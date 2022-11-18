use solang::Target;
use walkdir::WalkDir;

//const CODE_BLOCK: Lazy<Mutex<Regex>> = Lazy::new(|| {
//    Regex::new(r"\.\. code-block:: solidity:[\s\S]+?(?=\n\n\S)")
//        .unwrap()
//        .into()
//});

fn doc_example_files(dir: &str) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            let e = entry.unwrap();
            if let (true, Some(ext)) = (e.path().is_file(), e.path().extension()) {
                if ext == "sol" {
                    return Some(e.path().display().to_string());
                }
            }
            None
        })
        .collect()
}

fn assert_compile(path: &str, target: Target) {
    todo!()
}

#[test]
fn they_compile() {
    for p in doc_example_files("examples/") {
        println!("{p}")
    }
}
