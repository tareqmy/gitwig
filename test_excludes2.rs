use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("/Users/tareqmy/development/rustprojects/gitwig/target/checkout/.git/HEAD");
    let excludes = vec!["node_modules".to_string(), "target".to_string(), "checkout".to_string()];
    
    let is_excluded = excludes
        .iter()
        .any(|ex| path.components().any(|c| c.as_os_str() == ex.as_str()));
        
    println!("is_excluded: {}", is_excluded);
}
