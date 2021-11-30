use solang::{file_resolver::FileResolver, parse_and_resolve, Target};
use std::{
    fs::{read_dir, File},
    io::{self, Read},
    path::PathBuf,
};

#[test]
fn contract_tests() -> io::Result<()> {
    let targets = read_dir("tests/contract_testcases")?;

    for target in targets {
        let path = target?.path();

        if let Some(filename) = path.file_name() {
            if let Some(target) = Target::from(&filename.to_string_lossy()) {
                recurse_directory(path, target)?;
            }
        }
    }

    Ok(())
}

fn recurse_directory(path: PathBuf, target: Target) -> io::Result<()> {
    for entry in read_dir(path)? {
        let path = entry?.path();

        if path.is_dir() {
            recurse_directory(path, target)?;
        } else if let Some(ext) = path.extension() {
            if ext.to_string_lossy() == "sol" {
                parse_file(path, target)?;
            }
        }
    }

    Ok(())
}

fn parse_file(path: PathBuf, target: Target) -> io::Result<()> {
    let mut cache = FileResolver::new();

    let mut file = File::open(&path)?;

    let mut source = String::new();

    file.read_to_string(&mut source)?;

    // make sure the path uses unix file separators, this is what the dot file uses
    let filename = path.to_string_lossy().replace('\\', "/");

    println!("Parsing {} for {}", path.display(), target);

    // The files may have had their end of lines mangled on Windows
    cache.set_file_contents(&filename, source.replace("\r\n", "\n"));

    let ns = parse_and_resolve(&filename, &mut cache, target);

    let mut path = path;

    path.set_extension("dot");

    let generated_dot = ns.dotgraphviz();

    // use std::io::Write;

    // let mut file = File::create(&path)?;

    // file.write_all(generated_dot.as_bytes())?;

    let mut file = File::open(&path)?;

    let mut test_dot = String::new();

    file.read_to_string(&mut test_dot)?;

    // The dot files may have had their end of lines mangled on Windows
    let test_dot = test_dot.replace("\r\n", "\n");

    assert_eq!(generated_dot, test_dot);

    Ok(())
}
