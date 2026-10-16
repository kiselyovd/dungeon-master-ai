//! Legacy source-path marker.
//!
//! The production `app-server` binary is declared by the `app-bootstrap`
//! package. Keeping this file avoids breaking tooling bookmarks while making
//! it impossible for the compatibility crate to become a second composition
//! root accidentally.

fn main() {
    eprintln!("use the app-bootstrap package's app-server binary");
}
