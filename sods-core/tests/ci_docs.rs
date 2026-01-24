fn find_workspace_root() -> std::path::PathBuf {
    let mut current = std::env::current_dir().expect("Failed to get current dir");
    loop {
        if current.join("ARCHITECTURE.md").exists() {
            return current;
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            panic!("Could not find workspace root (ARCHITECTURE.md missing)");
        }
    }
}

#[test]
fn test_cmt_documentation_clarity() {
    let root = find_workspace_root();
    
    let arch = std::fs::read_to_string(root.join("ARCHITECTURE.md")).expect("Missing ARCHITECTURE.md");
    assert!(arch.contains("CMT") && arch.contains("PLANNED FOR V8.0"), "ARCHITECTURE.md should mark CMT as planned");
    assert!(arch.contains("BMT") && arch.contains("Active in v1.0 - v7.0"), "ARCHITECTURE.md should state BMT status");
    
    let readme = std::fs::read_to_string(root.join("README.md")).expect("Missing README.md");
    assert!(readme.contains("Causal Behavioral Proofs (Roadmap)"), "README.md should mark Causal Proofs as Roadmap");
    assert!(readme.contains("Planned for v8.0"), "README.md should state planned for v8.0");

    let spec = std::fs::read_to_string(root.join("spec/SODS-SPEC-v1.0.md")).expect("Missing SODS-SPEC-v1.0.md");
    assert!(spec.contains("Causal Behavioral Proof (Planned)"), "Spec should mark Causal Proofs as Planned");
}

#[test]
fn test_tree_comments_clarity() {
    let root = find_workspace_root();
    let tree_code = std::fs::read_to_string(root.join("sods-core/src/tree.rs")).expect("Missing tree.rs");
    assert!(tree_code.contains("This is NOT a Causal Merkle Tree (CMT)"), "tree.rs should clarify CMT status");
}
