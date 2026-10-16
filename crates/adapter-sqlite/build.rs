fn main() {
    // sqlx::migrate! must be rebuilt when a new migration file is added.
    println!("cargo:rerun-if-changed=migrations");
}
