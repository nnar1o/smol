use smol::config;
use smol::core::SummaryStatus;
use smol::parse;

// ---------- gh (GitHub CLI) ----------

#[test]
fn test_gh_success_parser() {
    let output = "\
Showing 2 of 2 open pull requests in owner/repo

NUMBER  TITLE                    BRANCH          STATE
#42     Fix bug in auth handler  fix-auth        OPEN
#43     Add logging middleware    feat-logging    OPEN
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("gh pr list", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_gh_failure_parser() {
    let output = "\
submitting pull request...
GraphQL: Could not resolve to a PullRequest with the current repository
HTTP 422: Unprocessable Entity
Pull request already exists for branch fix-auth
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("gh pr create", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_gh_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/gh_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("gh pr list", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_gh_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/gh_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("gh pr create", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- aws (AWS CLI) ----------

#[test]
fn test_aws_success_parser() {
    let output = "\
2024-01-15 10:30:00 my-bucket-1
2024-02-20 14:22:10 my-bucket-2
2024-03-10 09:15:45 logs-bucket
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("aws s3 ls", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_aws_failure_parser() {
    let output = "\
An error occurred (AccessDenied) when calling the PutObject operation: Access Denied. \
User: arn:aws:iam::123456789012:user/deploy-bot is not authorized to perform: s3:PutObject
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("aws s3 cp file.txt s3://my-bucket/", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 1, "Should detect at least 1 error, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_aws_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/aws_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("aws s3 ls", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_aws_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/aws_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("aws s3 cp file.txt s3://my-bucket/", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- helm (Kubernetes package manager) ----------

#[test]
fn test_helm_success_parser() {
    let output = "\
Release \"my-release\" does not exist. Installing it now.
NAME: my-release
LAST DEPLOYED: Thu Jun 11 10:00:00 2026
NAMESPACE: default
STATUS: deployed
REVISION: 1
TEST SUITE: None
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("helm install my-release ./mychart", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_helm_failure_parser() {
    let output = "\
Error: failed to download \"https://charts.example.com/mychart-1.0.0.tgz\" (hint: repo may be missing, try `helm repo add`)

Use \"helm list\" to see the list of deployed releases
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("helm install my-release ./mychart", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 1, "Should detect at least 1 error, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_helm_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/helm_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("helm install my-release ./mychart", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_helm_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/helm_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("helm install my-release ./mychart", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- ansible (Ansible automation) ----------

#[test]
fn test_ansible_success_parser() {
    let output = "\
PLAY [Deploy app] **************************************************************

TASK [Gathering Facts] *********************************************************
ok: [web-1]
ok: [web-2]

TASK [Install packages] ********************************************************
changed: [web-1]
changed: [web-2]

PLAY RECAP *********************************************************************
web-1 : ok=3 changed=1 unreachable=0 failed=0 skipped=0 rescued=0 ignored=0
web-2 : ok=3 changed=1 unreachable=0 failed=0 skipped=0 rescued=0 ignored=0
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("ansible-playbook deploy.yml", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_ansible_failure_parser() {
    let output = "\
PLAY [Deploy app] **************************************************************

TASK [Gathering Facts] *********************************************************
fatal: [web-1]: UNREACHABLE! => {\"changed\": false, \"msg\": \"Failed to connect\", \"unreachable\": true}
fatal: [web-2]: UNREACHABLE! => {\"changed\": false, \"msg\": \"Failed to connect\", \"unreachable\": true}

PLAY RECAP *********************************************************************
web-1 : ok=0 changed=0 unreachable=1 failed=1 skipped=0 rescued=0 ignored=0
web-2 : ok=0 changed=0 unreachable=1 failed=1 skipped=0 rescued=0 ignored=0
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("ansible-playbook deploy.yml", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_ansible_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/ansible_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("ansible-playbook deploy.yml", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_ansible_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/ansible_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("ansible-playbook deploy.yml", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
}

// ---------- docker-compose (Docker Compose) ----------

#[test]
fn test_docker_compose_success_parser() {
    let output = "\
Creating network \"myapp_default\" with the default driver
Creating volume \"myapp_data\" with default driver
Creating myapp-web-1    ... done
Creating myapp-db-1     ... done
Creating myapp-redis-1  ... done
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("docker-compose up -d", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.input_tokens > 0);
}

#[test]
fn test_docker_compose_failure_parser() {
    let output = "\
WARNING: The SOME_VAR variable is not set. Defaulting to a blank string.
Creating myapp-web-1    ... error
Creating myapp-db-1     ... done

ERROR: for myapp-web-1  Cannot start service web: driver failed programming external connectivity
Error response from daemon: driver failed programming external connectivity on endpoint myapp-web-1
";

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("docker-compose up", output, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count >= 2, "Should detect at least 2 errors, got {}", summary.error_count);
    assert!(summary.warning_count >= 1, "Should detect at least 1 warning, got {}", summary.warning_count);
}

#[test]
fn test_docker_compose_from_fixture_success() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/docker_compose_success.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("docker-compose up -d", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Success);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.warning_count, 0);
}

#[test]
fn test_docker_compose_from_fixture_failure() {
    let content =
        std::fs::read_to_string("tests/fixtures/outputs/docker_compose_failure.txt")
            .expect("fixture file not found");

    let parsers = config::load_all_parsers("parsers").unwrap();
    let summary = parse::parse_output("docker-compose up", &content, "", &parsers, 5, 10).unwrap();

    assert_eq!(summary.status, SummaryStatus::Failure);
    assert!(summary.error_count > 0, "Should detect errors");
    assert!(summary.warning_count > 0, "Should detect warnings");
}
