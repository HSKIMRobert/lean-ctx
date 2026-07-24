use super::*;
#[test]
fn gh_api_is_verbatim() {
    assert!(is_verbatim_output("gh api repos/owner/repo/issues/198"));
    assert!(is_verbatim_output("gh api repos/owner/repo/pulls/42"));
    assert!(is_verbatim_output(
        "gh api repos/owner/repo/issues/198 --jq '.body'"
    ));
}

#[test]
fn gh_json_and_jq_flags_are_verbatim() {
    assert!(is_verbatim_output("gh pr list --json number,title"));
    assert!(is_verbatim_output("gh issue list --jq '.[]'"));
    assert!(is_verbatim_output("gh pr view 42 --json body --jq '.body'"));
    assert!(is_verbatim_output("gh pr view 5 --template '{{.body}}'"));
}

#[test]
fn gh_search_and_release_verbatim() {
    assert!(is_verbatim_output("gh search repos lean-ctx"));
    assert!(is_verbatim_output("gh release view v3.5.18"));
    assert!(is_verbatim_output("gh gist view abc123"));
    assert!(is_verbatim_output("gh gist list"));
}

#[test]
fn gh_run_log_verbatim() {
    assert!(is_verbatim_output("gh run view 12345 --log"));
    assert!(is_verbatim_output("gh run view 12345 --log-failed"));
}

#[test]
fn glab_api_is_verbatim() {
    assert!(is_verbatim_output("glab api projects/123/issues"));
}

#[test]
fn jira_linear_verbatim() {
    assert!(is_verbatim_output("jira issue view PROJ-42"));
    assert!(is_verbatim_output("jira issue list"));
    assert!(is_verbatim_output("linear issue list"));
}

#[test]
fn saas_cli_data_commands_verbatim() {
    assert!(is_verbatim_output("stripe charges list"));
    assert!(is_verbatim_output("vercel logs my-deploy"));
    assert!(is_verbatim_output("fly status"));
    assert!(is_verbatim_output("railway logs"));
    assert!(is_verbatim_output("heroku logs --tail"));
    assert!(is_verbatim_output("heroku config"));
}

#[test]
fn gh_pr_create_not_verbatim() {
    assert!(!is_verbatim_output("gh pr create --title 'Fix bug'"));
    assert!(!is_verbatim_output("gh issue create --body 'desc'"));
}

#[test]
fn gh_api_pipe_is_verbatim() {
    assert!(is_verbatim_output(
        "gh api repos/owner/repo/pulls/42 | jq '.body'"
    ));
}
