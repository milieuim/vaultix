mod parser {
    pub mod permission;
    pub mod template;
}
pub use parser::permission::parse_permissions_str;
pub use parser::template::extract_all_hashes;
