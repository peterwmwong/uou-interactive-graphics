use std::{
    path::Path,
    process::{ChildStdout, Command, Stdio},
};

#[inline]
pub fn generate_metal_ast<P: AsRef<Path>, T, F: FnOnce(&mut ChildStdout) -> T>(
    shader_file: P,
    fun: F,
) -> T {
    let mut cmd = Command::new("xcrun")
        .args(&[
            "-sdk",
            "macosx",
            "metal",
            "-std=metal3.0",
            &shader_file.as_ref().to_string_lossy(),
            "-Xclang",
            "-ast-dump",
            "-fsyntax-only",
            "-fno-color-diagnostics",
        ])
        .env_clear()
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn metal command");
    let stdout = cmd
        .stdout
        .as_mut()
        .expect("Failed to access metal command output");
    let result = fun(stdout);
    cmd.wait().unwrap();
    result
}
