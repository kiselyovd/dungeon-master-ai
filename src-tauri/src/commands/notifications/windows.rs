//! Windows identity and COM activation are confined to the native adapter.
use super::{activate, app_scope, store, Event, Permission};
use std::ffi::c_void;
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use windows::{
    core::{implement, IUnknown, Interface, Ref, BOOL, GUID, HSTRING, PCWSTR},
    Data::Xml::Dom::XmlDocument,
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_INVALIDARG, E_POINTER, PROPERTYKEY},
        System::Com::{
            CoCreateInstance, CoRegisterClassObject, CoRevokeClassObject, IClassFactory,
            IClassFactory_Impl, IPersistFile,
            StructuredStorage::{InitPropVariantFromCLSID, PROPVARIANT},
            CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE,
        },
        UI::{
            Notifications::{
                INotificationActivationCallback, INotificationActivationCallback_Impl,
                NOTIFICATION_USER_INPUT_DATA,
            },
            Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink},
        },
    },
    UI::Notifications::{ToastNotification, ToastNotificationManager},
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

#[implement(INotificationActivationCallback)]
struct Activator {
    app: AppHandle,
}
impl INotificationActivationCallback_Impl for Activator_Impl {
    fn Activate(
        &self,
        app_id: &PCWSTR,
        arguments: &PCWSTR,
        _data: *const NOTIFICATION_USER_INPUT_DATA,
        count: u32,
    ) -> windows::core::Result<()> {
        if count != 0 {
            return Err(E_INVALIDARG.into());
        }
        // COM supplies valid null-terminated strings. Read a strict bounded prefix.
        let id = unsafe { bounded_string(*app_id, 256) }?;
        let token = unsafe { bounded_string(*arguments, 36) }?;
        if id != self.app.config().identifier {
            return Err(E_INVALIDARG.into());
        }
        let token =
            Uuid::parse_str(&token).map_err(|_| windows::core::Error::from(E_INVALIDARG))?;
        let app = self.app.clone();
        self.app
            .run_on_main_thread(move || {
                let Ok(store) = store(&app) else {
                    return;
                };
                let key = format!("activation:{token}");
                let event = store
                    .get(&key)
                    .and_then(|value| serde_json::from_value::<Event>(value).ok());
                // Remove before action so repeated COM delivery cannot activate twice.
                store.delete(&key);
                if store.save().is_err() {
                    return;
                }
                if let Some(event) = event {
                    activate(&app, &event);
                }
            })
            .map_err(|_| windows::core::Error::from(E_INVALIDARG))?;
        Ok(())
    }
}
unsafe fn bounded_string(value: PCWSTR, max: usize) -> windows::core::Result<String> {
    if value.is_null() {
        return Err(E_POINTER.into());
    }
    for length in 0..=max {
        // SAFETY: the COM contract provides a valid null-terminated string.
        if unsafe { *value.0.add(length) } == 0 {
            return String::from_utf16(unsafe { std::slice::from_raw_parts(value.0, length) })
                .map_err(|_| E_INVALIDARG.into());
        }
    }
    Err(E_INVALIDARG.into())
}
#[implement(IClassFactory)]
struct Factory {
    app: AppHandle,
}
impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        iid: *const GUID,
        output: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if !outer.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        if iid.is_null() || output.is_null() {
            return Err(E_POINTER.into());
        }
        let callback: INotificationActivationCallback = Activator {
            app: self.app.clone(),
        }
        .into();
        // SAFETY: valid COM output and interface-ID pointers checked above.
        unsafe {
            *output = std::ptr::null_mut();
            callback.query(iid, output).ok()
        }
    }
    fn LockServer(&self, _lock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}
thread_local! { static REGISTRATION: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) }; }

fn clsid(app: &AppHandle) -> Result<GUID, &'static str> {
    // The durable app scope gives each QA/production profile its own COM identity.
    app_scope(app).map(|id| GUID::from_u128(id.as_u128()))
}
pub(super) fn resume(app: &AppHandle) {
    if store(app)
        .ok()
        .and_then(|s| s.get("enabled"))
        .and_then(|v| v.as_bool())
        == Some(true)
        && enable(app).is_err()
    {
        log::warn!("native notification action=activation_unavailable");
    }
}
fn register_factory(app: &AppHandle) -> Result<(), &'static str> {
    REGISTRATION.with(|registration| {
        if registration.get().is_some() {
            return Ok(());
        }
        let factory: IClassFactory = Factory { app: app.clone() }.into();
        // SAFETY: the COM runtime retains the factory; cookie is revoked on Exit.
        let cookie = unsafe {
            CoRegisterClassObject(
                &clsid(app)?,
                &factory,
                CLSCTX_LOCAL_SERVER,
                REGCLS_MULTIPLEUSE,
            )
        }
        .map_err(|_| "native_notification_activation_unavailable")?;
        registration.set(Some(cookie));
        Ok(())
    })
}
pub(super) fn shutdown() {
    REGISTRATION.with(|registration| {
        if let Some(cookie) = registration.take() {
            // SAFETY: cookie is created and consumed on the same native thread.
            let _ = unsafe { CoRevokeClassObject(cookie) };
        }
    });
}
pub(super) fn enable(app: &AppHandle) -> Result<(), &'static str> {
    let exe = std::env::current_exe().map_err(|_| "native_notification_registration_failed")?;
    let id = &app.config().identifier;
    let class = clsid(app)?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let aumid = hkcu
        .create_subkey(format!("Software\\Classes\\AppUserModelId\\{id}"))
        .map_err(|_| "native_notification_registration_failed")?
        .0;
    aumid
        .set_value(
            "DisplayName",
            &app.config()
                .product_name
                .as_deref()
                .unwrap_or("Dungeon Master AI"),
        )
        .map_err(|_| "native_notification_registration_failed")?;
    aumid
        .set_value("CustomActivator", &format!("{{{class:?}}}"))
        .map_err(|_| "native_notification_registration_failed")?;
    let server = hkcu
        .create_subkey(format!(
            "Software\\Classes\\CLSID\\{{{class:?}}}\\LocalServer32"
        ))
        .map_err(|_| "native_notification_registration_failed")?
        .0;
    server
        .set_value(
            "",
            &format!("\"{}\" --notification-activated", exe.display()),
        )
        .map_err(|_| "native_notification_registration_failed")?;
    // A dedicated identity-named shortcut cannot overwrite the installer/user shortcut.
    let roaming = app
        .path()
        .app_data_dir()
        .map_err(|_| "native_notification_registration_failed")?;
    let shortcut = roaming
        .parent()
        .ok_or("native_notification_registration_failed")?
        .join("Microsoft/Windows/Start Menu/Programs")
        .join(format!("Dungeon Master AI - {id}.lnk"));
    let result = (|| -> windows::core::Result<()> {
        // SAFETY: native COM shell interfaces and owned property variants.
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            link.SetPath(&HSTRING::from(exe.to_string_lossy().as_ref()))?;
            let properties: IPropertyStore = link.cast()?;
            let mut key = PROPERTYKEY {
                fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
                pid: 5,
            };
            properties.SetValue(&key, &PROPVARIANT::from(id.as_str()))?;
            key.pid = 26;
            properties.SetValue(&key, &InitPropVariantFromCLSID(&class)?)?;
            properties.Commit()?;
            link.cast::<IPersistFile>()?
                .Save(&HSTRING::from(shortcut.to_string_lossy().as_ref()), true)?;
        }
        Ok(())
    })();
    result.map_err(|_| "native_notification_registration_failed")?;
    register_factory(app)?;
    // Identity-less Windows apps need one suppressed enrollment toast before Setting exists.
    // Never attempt this when Windows already reports explicit refusal.
    if super::permission(app) == Permission::Unknown {
        let result = (|| -> windows::core::Result<()> {
            let xml = XmlDocument::new()?;
            xml.LoadXml(&HSTRING::from("<toast><visual><binding template='ToastGeneric'><text>Dungeon Master AI</text></binding></visual></toast>"))?;
            let toast = ToastNotification::CreateToastNotification(&xml)?;
            toast.SetSuppressPopup(true)?;
            toast.SetTag(&HSTRING::from("dmai-enrollment"))?;
            toast.SetGroup(&HSTRING::from("dmai"))?;
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(id))?
                .Show(&toast)?;
            ToastNotificationManager::History()?.RemoveGroupedTagWithId(
                &HSTRING::from("dmai-enrollment"),
                &HSTRING::from("dmai"),
                &HSTRING::from(id),
            )?;
            Ok(())
        })();
        result.map_err(|_| "native_notification_registration_failed")?;
    }
    Ok(())
}
pub(super) fn show(app: &AppHandle, event: &Event, body: &str) -> Result<(), &'static str> {
    let token = Uuid::new_v4();
    let store = store(app)?;
    // Bounded typed outbox; never store narration or invitation payloads here.
    let mut keys: Vec<_> = store
        .keys()
        .into_iter()
        .filter(|key| key.starts_with("activation:"))
        .collect();
    keys.sort();
    for key in keys.iter().take(keys.len().saturating_sub(31)) {
        store.delete(key);
    }
    let key = format!("activation:{token}");
    store.set(
        &key,
        serde_json::to_value(event).map_err(|_| "native_notification_invalid")?,
    );
    store
        .save()
        .map_err(|_| "native_notification_storage_unavailable")?;
    let result = (|| -> windows::core::Result<()> {
        let xml = XmlDocument::new()?;
        xml.LoadXml(&HSTRING::from(format!("<toast launch='{token}'><visual><binding template='ToastGeneric'><text>Dungeon Master AI</text><text>{}</text></binding></visual></toast>", escape(body))))?;
        let toast = ToastNotification::CreateToastNotification(&xml)?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            &app.config().identifier,
        ))?
        .Show(&toast)
    })();
    if result.is_err() {
        store.delete(&key);
        let _ = store.save();
    }
    result.map_err(|_| "native_notification_delivery_failed")
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
