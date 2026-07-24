use super::*;
#[test]
fn git_diff_is_structural() {
    assert!(has_structural_output("git diff"));
    assert!(has_structural_output("git diff --cached"));
    assert!(has_structural_output("git diff --staged"));
    assert!(has_structural_output("git diff HEAD~1"));
    assert!(has_structural_output("git diff main..feature"));
    assert!(has_structural_output("git diff -- src/main.rs"));
}

#[test]
fn git_show_is_structural() {
    assert!(has_structural_output("git show"));
    assert!(has_structural_output("git show HEAD"));
    assert!(has_structural_output("git show abc1234"));
    assert!(has_structural_output("git show stash@{0}"));
}

#[test]
fn git_blame_is_structural() {
    assert!(has_structural_output("git blame src/main.rs"));
    assert!(has_structural_output("git blame -L 10,20 file.rs"));
}

#[test]
fn git_with_flags_is_structural() {
    assert!(has_structural_output("git -C /tmp diff"));
    assert!(has_structural_output("git --git-dir /path diff HEAD"));
    assert!(has_structural_output("git -c core.pager=cat show abc"));
}

#[test]
fn case_insensitive() {
    assert!(has_structural_output("Git Diff"));
    assert!(has_structural_output("GIT DIFF --cached"));
    assert!(has_structural_output("git SHOW HEAD"));
}

#[test]
fn full_path_git_binary() {
    assert!(has_structural_output("/usr/bin/git diff"));
    assert!(has_structural_output("/usr/local/bin/git show HEAD"));
}

#[test]
fn standalone_diff_is_structural() {
    assert!(has_structural_output("diff file1.txt file2.txt"));
    assert!(has_structural_output("diff -u old.py new.py"));
    assert!(has_structural_output("diff -r dir1 dir2"));
    assert!(has_structural_output("/usr/bin/diff a b"));
    assert!(has_structural_output("colordiff file1 file2"));
    assert!(has_structural_output("icdiff old.rs new.rs"));
    assert!(has_structural_output("delta"));
}

#[test]
fn git_log_with_patch_is_structural() {
    assert!(has_structural_output("git log -p"));
    assert!(has_structural_output("git log --patch"));
    assert!(has_structural_output("git log -p HEAD~5"));
    assert!(has_structural_output("git log -p --stat"));
    assert!(has_structural_output("git log --patch --follow file.rs"));
}

#[test]
fn git_log_without_patch_not_structural() {
    assert!(!has_structural_output("git log"));
    assert!(!has_structural_output("git log --oneline"));
    assert!(!has_structural_output("git log -n 5"));
}

#[test]
fn git_log_with_stat_is_structural() {
    assert!(has_structural_output("git log --stat"));
    assert!(has_structural_output("git log --stat -n 5"));
}

#[test]
fn git_stash_show_is_structural() {
    assert!(has_structural_output("git stash show"));
    assert!(has_structural_output("git stash show -p"));
    assert!(has_structural_output("git stash show --patch"));
    assert!(has_structural_output("git stash show stash@{0}"));
}

#[test]
fn git_stash_without_show_not_structural() {
    assert!(!has_structural_output("git stash"));
    assert!(!has_structural_output("git stash list"));
    assert!(!has_structural_output("git stash pop"));
    assert!(!has_structural_output("git stash drop"));
}

#[test]
fn non_structural_git_commands() {
    assert!(!has_structural_output("git status"));
    assert!(!has_structural_output("git fetch"));
    assert!(!has_structural_output("git add ."));
}

#[test]
fn git_write_commands_are_verbatim() {
    assert!(has_structural_output("git commit -m 'fix'"));
    assert!(has_structural_output("git push"));
    assert!(has_structural_output("git pull"));
    assert!(has_structural_output("git merge feature"));
    assert!(has_structural_output("git rebase main"));
    assert!(has_structural_output("git cherry-pick abc1234"));
    assert!(has_structural_output("git tag v1.0"));
    assert!(has_structural_output("git reset --hard HEAD~1"));
}

#[test]
fn non_git_commands() {
    assert!(!has_structural_output("cargo build"));
    assert!(!has_structural_output("npm run build"));
}

#[test]
fn verbatim_commands_are_also_structural() {
    assert!(has_structural_output("ls -la"));
    assert!(has_structural_output("docker ps"));
    assert!(has_structural_output("curl https://api.example.com"));
    assert!(has_structural_output("cat file.txt"));
    assert!(has_structural_output("aws ec2 describe-instances"));
    assert!(has_structural_output("npm list"));
    assert!(has_structural_output("node --version"));
    assert!(has_structural_output("journalctl -u nginx"));
    assert!(has_structural_output("git remote -v"));
    assert!(has_structural_output("pbpaste"));
    assert!(has_structural_output("env"));
}

#[test]
fn git_diff_output_preserves_hunks() {
    let diff = "diff --git a/src/main.rs b/src/main.rs\n\
            index abc1234..def5678 100644\n\
            --- a/src/main.rs\n\
            +++ b/src/main.rs\n\
            @@ -1,5 +1,6 @@\n\
             fn main() {\n\
            +    println!(\"hello\");\n\
                 let x = 1;\n\
                 let y = 2;\n\
            -    let z = 3;\n\
            +    let z = x + y;\n\
             }";
    let result = compress_if_beneficial("git diff", diff);
    assert!(
        result.contains("+    println!"),
        "must preserve added lines, got: {result}"
    );
    assert!(
        result.contains("-    let z = 3;"),
        "must preserve removed lines, got: {result}"
    );
    assert!(
        result.contains("@@ -1,5 +1,6 @@"),
        "must preserve hunk headers, got: {result}"
    );
}

#[test]
fn git_diff_large_preserves_content() {
    let mut diff = String::new();
    diff.push_str("diff --git a/file.rs b/file.rs\n");
    diff.push_str("--- a/file.rs\n+++ b/file.rs\n");
    diff.push_str("@@ -1,100 +1,100 @@\n");
    for i in 0..80 {
        diff.push_str(&format!("+added line {i}: some actual code content\n"));
        diff.push_str(&format!("-removed line {i}: old code content\n"));
    }
    let result = compress_if_beneficial("git diff", &diff);
    assert!(
        result.contains("+added line 0"),
        "must preserve first added line, got len: {}",
        result.len()
    );
    assert!(
        result.contains("-removed line 0"),
        "must preserve first removed line, got len: {}",
        result.len()
    );
}
