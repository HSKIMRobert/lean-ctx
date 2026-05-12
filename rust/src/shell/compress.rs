use crate::core::patterns;
use crate::core::tokens::count_tokens;

const BUILTIN_PASSTHROUGH: &[&str] = &[
    // lean-ctx itself — never compress our own output
    "lean-ctx",
    // JS/TS dev servers & watchers
    "turbo",
    "nx serve",
    "nx dev",
    "next dev",
    "vite dev",
    "vite preview",
    "vitest",
    "nuxt dev",
    "astro dev",
    "webpack serve",
    "webpack-dev-server",
    "nodemon",
    "concurrently",
    "pm2",
    "pm2 logs",
    "gatsby develop",
    "expo start",
    "react-scripts start",
    "ng serve",
    "remix dev",
    "wrangler dev",
    "hugo server",
    "hugo serve",
    "jekyll serve",
    "bun dev",
    "ember serve",
    // Package manager script runners (wrap dev servers via package.json)
    "npm run dev",
    "npm run start",
    "npm run serve",
    "npm run watch",
    "npm run preview",
    "npm run storybook",
    "npm run test:watch",
    "npm start",
    "npx ",
    "pnpm run dev",
    "pnpm run start",
    "pnpm run serve",
    "pnpm run watch",
    "pnpm run preview",
    "pnpm run storybook",
    "pnpm dev",
    "pnpm start",
    "pnpm preview",
    "yarn dev",
    "yarn start",
    "yarn serve",
    "yarn watch",
    "yarn preview",
    "yarn storybook",
    "bun run dev",
    "bun run start",
    "bun run serve",
    "bun run watch",
    "bun run preview",
    "bun start",
    "deno task dev",
    "deno task start",
    "deno task serve",
    "deno run --watch",
    // Docker
    "docker compose up",
    "docker-compose up",
    "docker compose logs",
    "docker-compose logs",
    "docker compose exec",
    "docker-compose exec",
    "docker compose run",
    "docker-compose run",
    "docker compose watch",
    "docker-compose watch",
    "docker logs",
    "docker attach",
    "docker exec -it",
    "docker exec -ti",
    "docker run -it",
    "docker run -ti",
    "docker stats",
    "docker events",
    // Kubernetes
    "kubectl logs",
    "kubectl exec -it",
    "kubectl exec -ti",
    "kubectl attach",
    "kubectl port-forward",
    "kubectl proxy",
    // System monitors & streaming
    "top",
    "htop",
    "btop",
    "watch ",
    "tail -f",
    "tail -f ",
    "journalctl -f",
    "journalctl --follow",
    "dmesg -w",
    "dmesg --follow",
    "strace",
    "tcpdump",
    "ping ",
    "ping6 ",
    "traceroute",
    "mtr ",
    "nmap ",
    "iperf ",
    "iperf3 ",
    "ss -l",
    "netstat -l",
    "lsof -i",
    "socat ",
    // Editors & pagers
    "less",
    "more",
    "vim",
    "nvim",
    "vi ",
    "nano",
    "micro ",
    "helix ",
    "hx ",
    "emacs",
    // Terminal multiplexers
    "tmux",
    "screen",
    // Interactive shells & REPLs
    "ssh ",
    "telnet ",
    "nc ",
    "ncat ",
    "psql",
    "mysql",
    "sqlite3",
    "redis-cli",
    "mongosh",
    "mongo ",
    "python3 -i",
    "python -i",
    "irb",
    "rails console",
    "rails c ",
    "iex",
    // Python servers, workers, watchers
    "flask run",
    "uvicorn ",
    "gunicorn ",
    "hypercorn ",
    "daphne ",
    "django-admin runserver",
    "manage.py runserver",
    "python manage.py runserver",
    "python -m http.server",
    "python3 -m http.server",
    "streamlit run",
    "gradio ",
    "celery worker",
    "celery -a",
    "celery -b",
    "dramatiq ",
    "rq worker",
    "watchmedo ",
    "ptw ",
    "pytest-watch",
    // Ruby / Rails
    "rails server",
    "rails s",
    "puma ",
    "unicorn ",
    "thin start",
    "foreman start",
    "overmind start",
    "guard ",
    "sidekiq",
    "resque ",
    // PHP / Laravel
    "php artisan serve",
    "php -s ",
    "php artisan queue:work",
    "php artisan queue:listen",
    "php artisan horizon",
    "php artisan tinker",
    "sail up",
    // Java / JVM
    "./gradlew bootrun",
    "gradlew bootrun",
    "gradle bootrun",
    "./gradlew run",
    "mvn spring-boot:run",
    "./mvnw spring-boot:run",
    "mvnw spring-boot:run",
    "mvn quarkus:dev",
    "./mvnw quarkus:dev",
    "sbt run",
    "sbt ~compile",
    "lein run",
    "lein repl",
    // Go
    "go run ",
    "air ",
    "gin ",
    "realize start",
    "reflex ",
    "gowatch ",
    // .NET / C#
    "dotnet run",
    "dotnet watch",
    "dotnet ef",
    // Elixir / Erlang
    "mix phx.server",
    "iex -s mix",
    // Swift
    "swift run",
    "swift package ",
    "vapor serve",
    // Zig
    "zig build run",
    // Rust
    "cargo watch",
    "cargo run",
    "cargo leptos watch",
    "bacon ",
    // General watchers & task runners
    "make dev",
    "make serve",
    "make watch",
    "make run",
    "make start",
    "just dev",
    "just serve",
    "just watch",
    "just start",
    "just run",
    "task dev",
    "task serve",
    "task watch",
    "nix develop",
    "devenv up",
    // CI/CD & infrastructure (long-running)
    "act ",
    "skaffold dev",
    "tilt up",
    "garden dev",
    "telepresence ",
    // Load testing & benchmarking
    "ab ",
    "wrk ",
    "hey ",
    "vegeta ",
    "k6 run",
    "artillery run",
    // Authentication flows (device code, OAuth, SSO)
    "az login",
    "az account",
    "gh",
    "gcloud auth",
    "gcloud init",
    "aws sso",
    "aws configure sso",
    "firebase login",
    "netlify login",
    "vercel login",
    "heroku login",
    "flyctl auth",
    "fly auth",
    "railway login",
    "supabase login",
    "wrangler login",
    "doppler login",
    "vault login",
    "oc login",
    "kubelogin",
    "--use-device-code",
];

const SCRIPT_RUNNER_PREFIXES: &[&str] = &[
    "npm run ",
    "npm start",
    "npx ",
    "pnpm run ",
    "pnpm dev",
    "pnpm start",
    "pnpm preview",
    "yarn ",
    "bun run ",
    "bun start",
    "deno task ",
];

const DEV_SCRIPT_KEYWORDS: &[&str] = &[
    "dev",
    "start",
    "serve",
    "watch",
    "preview",
    "storybook",
    "hot",
    "live",
    "hmr",
];

fn is_dev_script_runner(cmd: &str) -> bool {
    for prefix in SCRIPT_RUNNER_PREFIXES {
        if let Some(rest) = cmd.strip_prefix(prefix) {
            let script_name = rest.split_whitespace().next().unwrap_or("");
            for kw in DEV_SCRIPT_KEYWORDS {
                if script_name.contains(kw) {
                    return true;
                }
            }
        }
    }
    false
}

pub(super) fn is_excluded_command(command: &str, excluded: &[String]) -> bool {
    let cmd = command.trim().to_lowercase();
    for pattern in BUILTIN_PASSTHROUGH {
        if pattern.starts_with("--") {
            if cmd.contains(pattern) {
                return true;
            }
        } else if pattern.ends_with(' ') || pattern.ends_with('\t') {
            if cmd == pattern.trim() || cmd.starts_with(pattern) {
                return true;
            }
        } else if cmd == *pattern
            || cmd.starts_with(&format!("{pattern} "))
            || cmd.starts_with(&format!("{pattern}\t"))
            || cmd.contains(&format!(" {pattern} "))
            || cmd.contains(&format!(" {pattern}\t"))
            || cmd.contains(&format!("|{pattern} "))
            || cmd.contains(&format!("|{pattern}\t"))
            || cmd.ends_with(&format!(" {pattern}"))
            || cmd.ends_with(&format!("|{pattern}"))
        {
            return true;
        }
    }

    if is_dev_script_runner(&cmd) {
        return true;
    }

    if excluded.is_empty() {
        return false;
    }
    excluded.iter().any(|excl| {
        let excl_lower = excl.trim().to_lowercase();
        cmd == excl_lower || cmd.starts_with(&format!("{excl_lower} "))
    })
}

pub(super) fn compress_and_measure(command: &str, stdout: &str, stderr: &str) -> (String, usize) {
    let compressed_stdout = compress_if_beneficial(command, stdout);
    let compressed_stderr = compress_if_beneficial(command, stderr);

    let mut result = String::new();
    if !compressed_stdout.is_empty() {
        result.push_str(&compressed_stdout);
    }
    if !compressed_stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&compressed_stderr);
    }

    let content_for_counting = if let Some(pos) = result.rfind("\n[lean-ctx: ") {
        &result[..pos]
    } else {
        &result
    };
    let output_tokens = count_tokens(content_for_counting);
    (result, output_tokens)
}

fn is_search_output(command: &str) -> bool {
    let c = command.trim_start();
    c.starts_with("grep ")
        || c.starts_with("rg ")
        || c.starts_with("find ")
        || c.starts_with("fd ")
        || c.starts_with("ag ")
        || c.starts_with("ack ")
}

/// Returns true for commands whose output structure is critical for developer
/// readability. Pattern compression (light cleanup like removing `index` lines
/// or limiting context) still applies, but the terse pipeline and generic
/// compressors are skipped so diff hunks, blame annotations, etc. remain
/// fully readable.
pub fn has_structural_output(command: &str) -> bool {
    if is_verbatim_output(command) {
        return true;
    }
    if is_standalone_diff_command(command) {
        return true;
    }
    is_structural_git_command(command)
}

/// Returns true for commands where the output IS the purpose of the command.
/// These must never have their content transformed — only size-limited if huge.
/// Checks both the full command AND the last pipe segment for comprehensive coverage.
pub fn is_verbatim_output(command: &str) -> bool {
    is_verbatim_single(command) || is_verbatim_pipe_tail(command)
}

fn is_verbatim_single(command: &str) -> bool {
    is_http_client(command)
        || is_file_viewer(command)
        || is_data_format_tool(command)
        || is_binary_viewer(command)
        || is_infra_inspection(command)
        || is_crypto_command(command)
        || is_database_query(command)
        || is_dns_network_inspection(command)
        || is_language_one_liner(command)
        || is_container_listing(command)
        || is_file_listing(command)
        || is_system_query(command)
        || is_cloud_cli_query(command)
        || is_cli_api_data_command(command)
        || is_package_manager_info(command)
        || is_version_or_help(command)
        || is_config_viewer(command)
        || is_log_viewer(command)
        || is_archive_listing(command)
        || is_clipboard_tool(command)
        || is_git_data_command(command)
        || is_task_dry_run(command)
        || is_env_dump(command)
}

/// CLI tools that fetch or output raw API/structured data.
/// These MUST never be compressed -- compression destroys the payload.
fn is_cli_api_data_command(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();

    // gh (GitHub CLI) -- api, run view --log, search, release view, gist view
    if cl.starts_with("gh ")
        && (cl.starts_with("gh api ")
            || cl.starts_with("gh api\t")
            || cl.contains(" --json")
            || cl.contains(" --jq ")
            || cl.contains(" --template ")
            || (cl.contains("run view") && (cl.contains("--log") || cl.contains("log-failed")))
            || cl.starts_with("gh search ")
            || cl.starts_with("gh release view")
            || cl.starts_with("gh gist view")
            || cl.starts_with("gh gist list"))
    {
        return true;
    }

    // GitLab CLI (glab)
    if cl.starts_with("glab ") && cl.starts_with("glab api ") {
        return true;
    }

    // Jira CLI
    if cl.starts_with("jira ") && (cl.contains(" view") || cl.contains(" list")) {
        return true;
    }

    // Linear CLI
    if cl.starts_with("linear ") {
        return true;
    }

    // Stripe, Twilio, Vercel, Netlify, Fly, Railway, Supabase CLIs
    let first = first_binary(command);
    if matches!(
        first,
        "stripe" | "twilio" | "vercel" | "netlify" | "flyctl" | "fly" | "railway" | "supabase"
    ) && (cl.contains(" list")
        || cl.contains(" get")
        || cl.contains(" show")
        || cl.contains(" status")
        || cl.contains(" info")
        || cl.contains(" logs")
        || cl.contains(" inspect")
        || cl.contains(" export")
        || cl.contains(" describe"))
    {
        return true;
    }

    // Cloudflare (wrangler)
    if cl.starts_with("wrangler ")
        && !cl.starts_with("wrangler dev")
        && (cl.contains(" tail") || cl.contains(" secret list") || cl.contains(" kv "))
    {
        return true;
    }

    // Heroku
    if cl.starts_with("heroku ")
        && (cl.contains(" config")
            || cl.contains(" logs")
            || cl.contains(" ps")
            || cl.contains(" info"))
    {
        return true;
    }

    false
}

/// For piped commands like `kubectl get pods -o json | jq '.items[]'`,
/// check if the LAST command in the pipe is a verbatim tool.
fn is_verbatim_pipe_tail(command: &str) -> bool {
    if !command.contains('|') {
        return false;
    }
    let last_segment = command.rsplit('|').next().unwrap_or("").trim();
    if last_segment.is_empty() {
        return false;
    }
    is_verbatim_single(last_segment)
}

fn is_http_client(command: &str) -> bool {
    let first = first_binary(command);
    matches!(
        first,
        "curl" | "wget" | "http" | "https" | "xh" | "curlie" | "grpcurl" | "grpc_cli"
    )
}

fn is_file_viewer(command: &str) -> bool {
    let first = first_binary(command);
    match first {
        "cat" | "bat" | "batcat" | "pygmentize" | "highlight" => true,
        "head" | "tail" => !command.contains("-f") && !command.contains("--follow"),
        _ => false,
    }
}

fn is_data_format_tool(command: &str) -> bool {
    let first = first_binary(command);
    matches!(
        first,
        "jq" | "yq"
            | "xq"
            | "fx"
            | "gron"
            | "mlr"
            | "miller"
            | "dasel"
            | "csvlook"
            | "csvcut"
            | "csvgrep"
            | "csvjson"
            | "in2csv"
            | "sql2csv"
    )
}

fn is_binary_viewer(command: &str) -> bool {
    let first = first_binary(command);
    matches!(first, "xxd" | "hexdump" | "od" | "strings" | "file")
}

fn is_infra_inspection(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    if cl.starts_with("terraform output")
        || cl.starts_with("terraform show")
        || cl.starts_with("terraform state show")
        || cl.starts_with("terraform state list")
        || cl.starts_with("terraform state pull")
        || cl.starts_with("tofu output")
        || cl.starts_with("tofu show")
        || cl.starts_with("tofu state show")
        || cl.starts_with("tofu state list")
        || cl.starts_with("tofu state pull")
        || cl.starts_with("pulumi stack output")
        || cl.starts_with("pulumi stack export")
    {
        return true;
    }
    if cl.starts_with("docker inspect") || cl.starts_with("podman inspect") {
        return true;
    }
    if (cl.starts_with("kubectl get") || cl.starts_with("k get"))
        && (cl.contains("-o yaml")
            || cl.contains("-o json")
            || cl.contains("-oyaml")
            || cl.contains("-ojson")
            || cl.contains("--output yaml")
            || cl.contains("--output json")
            || cl.contains("--output=yaml")
            || cl.contains("--output=json"))
    {
        return true;
    }
    if cl.starts_with("kubectl describe") || cl.starts_with("k describe") {
        return true;
    }
    if cl.starts_with("helm get") || cl.starts_with("helm template") {
        return true;
    }
    false
}

fn is_crypto_command(command: &str) -> bool {
    let first = first_binary(command);
    if first == "openssl" {
        return true;
    }
    matches!(first, "gpg" | "age" | "ssh-keygen" | "certutil")
}

fn is_database_query(command: &str) -> bool {
    let cl = command.to_ascii_lowercase();
    if cl.starts_with("psql ") && (cl.contains(" -c ") || cl.contains("--command")) {
        return true;
    }
    if cl.starts_with("mysql ") && (cl.contains(" -e ") || cl.contains("--execute")) {
        return true;
    }
    if cl.starts_with("mariadb ") && (cl.contains(" -e ") || cl.contains("--execute")) {
        return true;
    }
    if cl.starts_with("sqlite3 ") && cl.contains('"') {
        return true;
    }
    if cl.starts_with("mongosh ") && cl.contains("--eval") {
        return true;
    }
    false
}

fn is_dns_network_inspection(command: &str) -> bool {
    let first = first_binary(command);
    matches!(
        first,
        "dig" | "nslookup" | "host" | "whois" | "drill" | "resolvectl"
    )
}

fn is_language_one_liner(command: &str) -> bool {
    let cl = command.to_ascii_lowercase();
    (cl.starts_with("python ") || cl.starts_with("python3 "))
        && (cl.contains(" -c ") || cl.contains(" -c\"") || cl.contains(" -c'"))
        || (cl.starts_with("node ") && (cl.contains(" -e ") || cl.contains(" --eval")))
        || (cl.starts_with("ruby ") && cl.contains(" -e "))
        || (cl.starts_with("perl ") && cl.contains(" -e "))
        || (cl.starts_with("php ") && cl.contains(" -r "))
}

fn is_container_listing(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    if cl.starts_with("docker ps") || cl.starts_with("docker images") {
        return true;
    }
    if cl.starts_with("podman ps") || cl.starts_with("podman images") {
        return true;
    }
    if cl.starts_with("kubectl get") || cl.starts_with("k get") {
        return true;
    }
    if cl.starts_with("helm list") || cl.starts_with("helm ls") {
        return true;
    }
    if cl.starts_with("docker compose ps") || cl.starts_with("docker-compose ps") {
        return true;
    }
    false
}

fn is_file_listing(command: &str) -> bool {
    let first = first_binary(command);
    matches!(
        first,
        "find" | "fd" | "fdfind" | "ls" | "exa" | "eza" | "lsd"
    )
}

fn is_system_query(command: &str) -> bool {
    let first = first_binary(command);
    matches!(
        first,
        "stat"
            | "wc"
            | "du"
            | "df"
            | "free"
            | "uname"
            | "id"
            | "whoami"
            | "hostname"
            | "uptime"
            | "lscpu"
            | "lsblk"
            | "ip"
            | "ifconfig"
            | "route"
            | "ss"
            | "netstat"
            | "base64"
            | "sha256sum"
            | "sha1sum"
            | "md5sum"
            | "cksum"
            | "readlink"
            | "realpath"
            | "which"
            | "type"
            | "command"
    )
}

fn is_cloud_cli_query(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    let cloud_query_verbs = [
        "describe",
        "get",
        "list",
        "show",
        "export",
        "inspect",
        "info",
        "status",
        "whoami",
        "caller-identity",
        "account",
    ];

    let is_aws = cl.starts_with("aws ") && !cl.starts_with("aws configure");
    let is_gcloud =
        cl.starts_with("gcloud ") && !cl.starts_with("gcloud auth") && !cl.contains(" deploy");
    let is_az = cl.starts_with("az ") && !cl.starts_with("az login");

    if !(is_aws || is_gcloud || is_az) {
        return false;
    }

    cloud_query_verbs
        .iter()
        .any(|verb| cl.contains(&format!(" {verb}")))
}

fn is_package_manager_info(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();

    if cl.starts_with("npm ") {
        return cl.starts_with("npm list")
            || cl.starts_with("npm ls")
            || cl.starts_with("npm info")
            || cl.starts_with("npm view")
            || cl.starts_with("npm show")
            || cl.starts_with("npm outdated")
            || cl.starts_with("npm audit");
    }
    if cl.starts_with("yarn ") {
        return cl.starts_with("yarn list")
            || cl.starts_with("yarn info")
            || cl.starts_with("yarn why")
            || cl.starts_with("yarn outdated")
            || cl.starts_with("yarn audit");
    }
    if cl.starts_with("pnpm ") {
        return cl.starts_with("pnpm list")
            || cl.starts_with("pnpm ls")
            || cl.starts_with("pnpm why")
            || cl.starts_with("pnpm outdated")
            || cl.starts_with("pnpm audit");
    }
    if cl.starts_with("pip ") || cl.starts_with("pip3 ") {
        return cl.contains(" list") || cl.contains(" show") || cl.contains(" freeze");
    }
    if cl.starts_with("gem ") {
        return cl.starts_with("gem list")
            || cl.starts_with("gem info")
            || cl.starts_with("gem specification");
    }
    if cl.starts_with("cargo ") {
        return cl.starts_with("cargo metadata")
            || cl.starts_with("cargo tree")
            || cl.starts_with("cargo pkgid");
    }
    if cl.starts_with("go ") {
        return cl.starts_with("go list") || cl.starts_with("go version");
    }
    if cl.starts_with("composer ") {
        return cl.starts_with("composer show")
            || cl.starts_with("composer info")
            || cl.starts_with("composer outdated");
    }
    if cl.starts_with("brew ") {
        return cl.starts_with("brew list")
            || cl.starts_with("brew info")
            || cl.starts_with("brew deps")
            || cl.starts_with("brew outdated");
    }
    if cl.starts_with("apt ") || cl.starts_with("dpkg ") {
        return cl.starts_with("apt list")
            || cl.starts_with("apt show")
            || cl.starts_with("dpkg -l")
            || cl.starts_with("dpkg --list")
            || cl.starts_with("dpkg -s");
    }
    false
}

fn is_version_or_help(command: &str) -> bool {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    parts.iter().any(|p| {
        *p == "--version"
            || *p == "-V"
            || p.eq_ignore_ascii_case("version")
            || *p == "--help"
            || *p == "-h"
            || p.eq_ignore_ascii_case("help")
    })
}

fn is_config_viewer(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    if cl.starts_with("git config") && !cl.contains("--set") && !cl.contains("--unset") {
        return true;
    }
    if cl.starts_with("npm config list") || cl.starts_with("npm config get") {
        return true;
    }
    if cl.starts_with("yarn config") && !cl.contains(" set") {
        return true;
    }
    if cl.starts_with("pip config list") || cl.starts_with("pip3 config list") {
        return true;
    }
    if cl.starts_with("rustup show") || cl.starts_with("rustup target list") {
        return true;
    }
    if cl.starts_with("docker context ls") || cl.starts_with("docker context list") {
        return true;
    }
    if cl.starts_with("kubectl config")
        && (cl.contains("view") || cl.contains("get-contexts") || cl.contains("current-context"))
    {
        return true;
    }
    false
}

fn is_log_viewer(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    if cl.starts_with("journalctl") && !cl.contains("-f") && !cl.contains("--follow") {
        return true;
    }
    if cl.starts_with("dmesg") && !cl.contains("-w") && !cl.contains("--follow") {
        return true;
    }
    if cl.starts_with("docker logs") && !cl.contains("-f") && !cl.contains("--follow") {
        return true;
    }
    if cl.starts_with("kubectl logs") && !cl.contains("-f") && !cl.contains("--follow") {
        return true;
    }
    if cl.starts_with("docker compose logs") && !cl.contains("-f") && !cl.contains("--follow") {
        return true;
    }
    false
}

fn is_archive_listing(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    if cl.starts_with("tar ") && (cl.contains(" -tf") || cl.contains(" -t") || cl.contains(" tf")) {
        return true;
    }
    if cl.starts_with("unzip -l") || cl.starts_with("unzip -Z") {
        return true;
    }
    let first = first_binary(command);
    matches!(first, "zipinfo" | "lsar" | "7z" if cl.contains(" l ") || cl.contains(" l\t"))
        || first == "zipinfo"
        || first == "lsar"
}

fn is_clipboard_tool(command: &str) -> bool {
    let first = first_binary(command);
    if matches!(first, "pbpaste" | "wl-paste") {
        return true;
    }
    let cl = command.trim().to_ascii_lowercase();
    if cl.starts_with("xclip") && cl.contains("-o") {
        return true;
    }
    if cl.starts_with("xsel") && (cl.contains("-o") || cl.contains("--output")) {
        return true;
    }
    false
}

fn is_git_data_command(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    if !cl.contains("git") {
        return false;
    }
    let exact_data_subs = [
        "remote",
        "rev-parse",
        "rev-list",
        "ls-files",
        "ls-tree",
        "ls-remote",
        "shortlog",
        "for-each-ref",
        "cat-file",
        "name-rev",
        "describe",
        "merge-base",
    ];

    let mut tokens = cl.split_whitespace();
    while let Some(tok) = tokens.next() {
        let base = tok.rsplit('/').next().unwrap_or(tok);
        if base != "git" {
            continue;
        }
        let mut skip_next = false;
        for arg in tokens.by_ref() {
            if skip_next {
                skip_next = false;
                continue;
            }
            if arg == "-c" || arg == "-C" || arg == "--git-dir" || arg == "--work-tree" {
                skip_next = true;
                continue;
            }
            if arg.starts_with('-') {
                continue;
            }
            return exact_data_subs.contains(&arg);
        }
        return false;
    }
    false
}

fn is_task_dry_run(command: &str) -> bool {
    let cl = command.trim().to_ascii_lowercase();
    if cl.starts_with("make ") && (cl.contains(" -n") || cl.contains(" --dry-run")) {
        return true;
    }
    if cl.starts_with("ansible") && (cl.contains("--check") || cl.contains("--diff")) {
        return true;
    }
    false
}

fn is_env_dump(command: &str) -> bool {
    let first = first_binary(command);
    matches!(first, "env" | "printenv" | "set" | "export" | "locale")
}

/// Extracts the binary name (basename, no path) from the first token of a command.
fn first_binary(command: &str) -> &str {
    let first = command.split_whitespace().next().unwrap_or("");
    first.rsplit('/').next().unwrap_or(first)
}

/// Non-git diff tools: `diff`, `colordiff`, `icdiff`, `delta`.
fn is_standalone_diff_command(command: &str) -> bool {
    let first = command.split_whitespace().next().unwrap_or("");
    let base = first.rsplit('/').next().unwrap_or(first);
    base.eq_ignore_ascii_case("diff")
        || base.eq_ignore_ascii_case("colordiff")
        || base.eq_ignore_ascii_case("icdiff")
        || base.eq_ignore_ascii_case("delta")
}

/// Git subcommands that produce structural output the developer must read verbatim.
fn is_structural_git_command(command: &str) -> bool {
    let mut tokens = command.split_whitespace();
    while let Some(tok) = tokens.next() {
        let base = tok.rsplit('/').next().unwrap_or(tok);
        if !base.eq_ignore_ascii_case("git") {
            continue;
        }
        let mut skip_next = false;
        let remaining: Vec<&str> = tokens.collect();
        for arg in &remaining {
            if skip_next {
                skip_next = false;
                continue;
            }
            if *arg == "-C" || *arg == "-c" || *arg == "--git-dir" || *arg == "--work-tree" {
                skip_next = true;
                continue;
            }
            if arg.starts_with('-') {
                continue;
            }
            let sub = arg.to_ascii_lowercase();
            return match sub.as_str() {
                "diff" | "show" | "blame" => true,
                "log" => has_patch_flag(&remaining),
                "stash" => remaining.iter().any(|a| a.eq_ignore_ascii_case("show")),
                _ => false,
            };
        }
        return false;
    }
    false
}

/// Returns true if the argument list contains `-p` or `--patch`.
fn has_patch_flag(args: &[&str]) -> bool {
    args.iter()
        .any(|a| *a == "-p" || *a == "--patch" || a.starts_with("-p"))
}

fn compress_if_beneficial(command: &str, output: &str) -> String {
    if output.trim().is_empty() {
        return String::new();
    }

    if !is_search_output(command) && crate::tools::ctx_shell::contains_auth_flow(output) {
        return output.to_string();
    }

    let original_tokens = count_tokens(output);

    if original_tokens < 50 {
        return output.to_string();
    }

    let min_output_tokens = 5;

    // OutputPolicy gate: if the command is classified as Verbatim,
    // only apply size-cap truncation — NEVER pattern compress or
    // run through the fallback chain (terse, cleanup, safety_scan).
    let cfg = crate::core::config::Config::load();
    let policy = super::output_policy::classify(command, &cfg.excluded_commands);
    if policy == super::output_policy::OutputPolicy::Verbatim
        || policy == super::output_policy::OutputPolicy::Passthrough
    {
        return truncate_verbatim(output, original_tokens);
    }

    if is_verbatim_output(command) {
        return truncate_verbatim(output, original_tokens);
    }

    if has_structural_output(command) {
        let cl = command.to_ascii_lowercase();
        if let Some(compressed) = patterns::try_specific_pattern(&cl, output) {
            if !compressed.trim().is_empty() {
                let compressed_tokens = count_tokens(&compressed);
                if compressed_tokens >= min_output_tokens && compressed_tokens < original_tokens {
                    let saved = original_tokens - compressed_tokens;
                    let pct = (saved as f64 / original_tokens as f64 * 100.0).round() as usize;
                    if pct >= 5 {
                        return format!(
                            "{compressed}\n[lean-ctx: {original_tokens}→{compressed_tokens} tok, -{pct}%]"
                        );
                    }
                    return compressed;
                }
            }
        }
        return output.to_string();
    }

    if let Some(mut compressed) = patterns::compress_output(command, output) {
        if !compressed.trim().is_empty() {
            let config = crate::core::config::Config::load();
            let level = crate::core::config::CompressionLevel::effective(&config);
            if level.is_active() {
                let terse_result =
                    crate::core::terse::pipeline::compress(output, &level, Some(&compressed));
                if terse_result.quality_passed {
                    compressed = terse_result.output;
                }
            }

            let compressed_tokens = count_tokens(&compressed);
            if compressed_tokens >= min_output_tokens && compressed_tokens < original_tokens {
                let ratio = compressed_tokens as f64 / original_tokens as f64;
                if ratio < 0.05 && original_tokens > 100 && original_tokens < 2000 {
                    tracing::warn!("compression removed >95% of small output, returning original");
                    return output.to_string();
                }
                let saved = original_tokens - compressed_tokens;
                let pct = (saved as f64 / original_tokens as f64 * 100.0).round() as usize;
                if pct >= 5 {
                    return format!(
                        "{compressed}\n[lean-ctx: {original_tokens}→{compressed_tokens} tok, -{pct}%]"
                    );
                }
                return compressed;
            }
            if compressed_tokens < min_output_tokens {
                return output.to_string();
            }
        }
    }

    {
        let config = crate::core::config::Config::load();
        let level = crate::core::config::CompressionLevel::effective(&config);
        if level.is_active() {
            let terse_result = crate::core::terse::pipeline::compress(output, &level, None);
            if terse_result.quality_passed && terse_result.savings_pct >= 3.0 {
                let tok_before = terse_result.tokens_before;
                let tok_after = terse_result.tokens_after;
                let pct = terse_result.savings_pct.round() as usize;
                return format!(
                    "{}\n[lean-ctx: {tok_before}→{tok_after} tok, -{pct}%]",
                    terse_result.output
                );
            }
        }
    }

    let cleaned = crate::core::compressor::lightweight_cleanup(output);
    let cleaned_tokens = count_tokens(&cleaned);
    if cleaned_tokens < original_tokens {
        let lines: Vec<&str> = cleaned.lines().collect();
        if lines.len() > 30 {
            let compressed = truncate_with_safety_scan(&lines, original_tokens);
            if let Some(c) = compressed {
                return c;
            }
        }
        if cleaned_tokens < original_tokens {
            let saved = original_tokens - cleaned_tokens;
            let pct = (saved as f64 / original_tokens as f64 * 100.0).round() as usize;
            if pct >= 5 {
                return format!(
                    "{cleaned}\n[lean-ctx: {original_tokens}→{cleaned_tokens} tok, -{pct}%]"
                );
            }
            return cleaned;
        }
    }

    let lines: Vec<&str> = output.lines().collect();
    if lines.len() > 30 {
        if let Some(c) = truncate_with_safety_scan(&lines, original_tokens) {
            return c;
        }
    }

    output.to_string()
}

const MAX_VERBATIM_TOKENS: usize = 8000;

/// For verbatim commands: never transform content, only head/tail truncate if huge.
fn truncate_verbatim(output: &str, original_tokens: usize) -> String {
    if original_tokens <= MAX_VERBATIM_TOKENS {
        return output.to_string();
    }
    let lines: Vec<&str> = output.lines().collect();
    let total = lines.len();
    if total <= 60 {
        return output.to_string();
    }
    let head = 30.min(total);
    let tail = 20.min(total.saturating_sub(head));
    let omitted = total - head - tail;
    let mut result = String::with_capacity(output.len() / 2);
    for line in &lines[..head] {
        result.push_str(line);
        result.push('\n');
    }
    result.push_str(&format!(
        "\n[{omitted} lines omitted — output too large for context window]\n\n"
    ));
    for line in lines.iter().skip(total - tail) {
        result.push_str(line);
        result.push('\n');
    }
    let truncated_tokens = count_tokens(&result);
    result.push_str(&format!(
        "[lean-ctx: {original_tokens}→{truncated_tokens} tok, verbatim truncated]"
    ));
    result
}

fn truncate_with_safety_scan(lines: &[&str], original_tokens: usize) -> Option<String> {
    use crate::core::safety_needles;

    let first = &lines[..5];
    let last = &lines[lines.len() - 5..];
    let middle = &lines[5..lines.len() - 5];

    let safety_lines = safety_needles::extract_safety_lines(middle, 20);
    let safety_count = safety_lines.len();
    let omitted = middle.len() - safety_count;

    let mut parts = Vec::new();
    parts.push(first.join("\n"));
    if safety_count > 0 {
        parts.push(format!(
            "[{omitted} lines omitted, {safety_count} safety-relevant lines preserved]"
        ));
        parts.push(safety_lines.join("\n"));
    } else {
        parts.push(format!("[{omitted} lines omitted]"));
    }
    parts.push(last.join("\n"));

    let compressed = parts.join("\n");
    let ct = count_tokens(&compressed);
    if ct >= original_tokens {
        return None;
    }
    let saved = original_tokens - ct;
    let pct = (saved as f64 / original_tokens as f64 * 100.0).round() as usize;
    if pct >= 5 {
        Some(format!(
            "{compressed}\n[lean-ctx: {original_tokens}→{ct} tok, -{pct}%]"
        ))
    } else {
        Some(compressed)
    }
}

/// Public wrapper for integration tests to exercise the compression pipeline.
pub fn compress_if_beneficial_pub(command: &str, output: &str) -> String {
    compress_if_beneficial(command, output)
}

#[cfg(test)]
mod passthrough_tests {
    use super::is_excluded_command;

    #[test]
    fn turbo_is_passthrough() {
        assert!(is_excluded_command("turbo run dev", &[]));
        assert!(is_excluded_command("turbo run build", &[]));
        assert!(is_excluded_command("pnpm turbo run dev", &[]));
        assert!(is_excluded_command("npx turbo run dev", &[]));
    }

    #[test]
    fn dev_servers_are_passthrough() {
        assert!(is_excluded_command("next dev", &[]));
        assert!(is_excluded_command("vite dev", &[]));
        assert!(is_excluded_command("nuxt dev", &[]));
        assert!(is_excluded_command("astro dev", &[]));
        assert!(is_excluded_command("nodemon server.js", &[]));
    }

    #[test]
    fn interactive_tools_are_passthrough() {
        assert!(is_excluded_command("vim file.rs", &[]));
        assert!(is_excluded_command("nvim", &[]));
        assert!(is_excluded_command("htop", &[]));
        assert!(is_excluded_command("ssh user@host", &[]));
        assert!(is_excluded_command("tail -f /var/log/syslog", &[]));
    }

    #[test]
    fn docker_streaming_is_passthrough() {
        assert!(is_excluded_command("docker logs my-container", &[]));
        assert!(is_excluded_command("docker logs -f webapp", &[]));
        assert!(is_excluded_command("docker attach my-container", &[]));
        assert!(is_excluded_command("docker exec -it web bash", &[]));
        assert!(is_excluded_command("docker exec -ti web bash", &[]));
        assert!(is_excluded_command("docker run -it ubuntu bash", &[]));
        assert!(is_excluded_command("docker compose exec web bash", &[]));
        assert!(is_excluded_command("docker stats", &[]));
        assert!(is_excluded_command("docker events", &[]));
    }

    #[test]
    fn kubectl_is_passthrough() {
        assert!(is_excluded_command("kubectl logs my-pod", &[]));
        assert!(is_excluded_command("kubectl logs -f deploy/web", &[]));
        assert!(is_excluded_command("kubectl exec -it pod -- bash", &[]));
        assert!(is_excluded_command(
            "kubectl port-forward svc/web 8080:80",
            &[]
        ));
        assert!(is_excluded_command("kubectl attach my-pod", &[]));
        assert!(is_excluded_command("kubectl proxy", &[]));
    }

    #[test]
    fn database_repls_are_passthrough() {
        assert!(is_excluded_command("psql -U user mydb", &[]));
        assert!(is_excluded_command("mysql -u root -p", &[]));
        assert!(is_excluded_command("sqlite3 data.db", &[]));
        assert!(is_excluded_command("redis-cli", &[]));
        assert!(is_excluded_command("mongosh", &[]));
    }

    #[test]
    fn streaming_tools_are_passthrough() {
        assert!(is_excluded_command("journalctl -f", &[]));
        assert!(is_excluded_command("ping 8.8.8.8", &[]));
        assert!(is_excluded_command("strace -p 1234", &[]));
        assert!(is_excluded_command("tcpdump -i eth0", &[]));
        assert!(is_excluded_command("tail -F /var/log/app.log", &[]));
        assert!(is_excluded_command("tmux new -s work", &[]));
        assert!(is_excluded_command("screen -S dev", &[]));
    }

    #[test]
    fn additional_dev_servers_are_passthrough() {
        assert!(is_excluded_command("gatsby develop", &[]));
        assert!(is_excluded_command("ng serve --port 4200", &[]));
        assert!(is_excluded_command("remix dev", &[]));
        assert!(is_excluded_command("wrangler dev", &[]));
        assert!(is_excluded_command("hugo server", &[]));
        assert!(is_excluded_command("bun dev", &[]));
        assert!(is_excluded_command("cargo watch -x test", &[]));
    }

    #[test]
    fn normal_commands_not_excluded() {
        assert!(!is_excluded_command("git status", &[]));
        assert!(!is_excluded_command("cargo test", &[]));
        assert!(!is_excluded_command("npm run build", &[]));
        assert!(!is_excluded_command("ls -la", &[]));
    }

    #[test]
    fn user_exclusions_work() {
        let excl = vec!["myapp".to_string()];
        assert!(is_excluded_command("myapp serve", &excl));
        assert!(!is_excluded_command("git status", &excl));
    }

    #[test]
    fn auth_commands_excluded() {
        assert!(is_excluded_command("az login --use-device-code", &[]));
        assert!(is_excluded_command("gh auth login", &[]));
        assert!(is_excluded_command("gh pr close --comment 'done'", &[]));
        assert!(is_excluded_command("gh issue list", &[]));
        assert!(is_excluded_command("gcloud auth login", &[]));
        assert!(is_excluded_command("aws sso login", &[]));
        assert!(is_excluded_command("firebase login", &[]));
        assert!(is_excluded_command("vercel login", &[]));
        assert!(is_excluded_command("heroku login", &[]));
        assert!(is_excluded_command("az login", &[]));
        assert!(is_excluded_command("kubelogin convert-kubeconfig", &[]));
        assert!(is_excluded_command("vault login -method=oidc", &[]));
        assert!(is_excluded_command("flyctl auth login", &[]));
    }

    #[test]
    fn auth_exclusion_does_not_affect_normal_commands() {
        assert!(!is_excluded_command("git log", &[]));
        assert!(!is_excluded_command("npm run build", &[]));
        assert!(!is_excluded_command("cargo test", &[]));
        assert!(!is_excluded_command("aws s3 ls", &[]));
        assert!(!is_excluded_command("gcloud compute instances list", &[]));
        assert!(!is_excluded_command("az vm list", &[]));
    }

    #[test]
    fn npm_script_runners_are_passthrough() {
        assert!(is_excluded_command("npm run dev", &[]));
        assert!(is_excluded_command("npm run start", &[]));
        assert!(is_excluded_command("npm run serve", &[]));
        assert!(is_excluded_command("npm run watch", &[]));
        assert!(is_excluded_command("npm run preview", &[]));
        assert!(is_excluded_command("npm run storybook", &[]));
        assert!(is_excluded_command("npm run test:watch", &[]));
        assert!(is_excluded_command("npm start", &[]));
        assert!(is_excluded_command("npx vite", &[]));
        assert!(is_excluded_command("npx next dev", &[]));
    }

    #[test]
    fn pnpm_script_runners_are_passthrough() {
        assert!(is_excluded_command("pnpm run dev", &[]));
        assert!(is_excluded_command("pnpm run start", &[]));
        assert!(is_excluded_command("pnpm run serve", &[]));
        assert!(is_excluded_command("pnpm run watch", &[]));
        assert!(is_excluded_command("pnpm run preview", &[]));
        assert!(is_excluded_command("pnpm dev", &[]));
        assert!(is_excluded_command("pnpm start", &[]));
        assert!(is_excluded_command("pnpm preview", &[]));
    }

    #[test]
    fn yarn_script_runners_are_passthrough() {
        assert!(is_excluded_command("yarn dev", &[]));
        assert!(is_excluded_command("yarn start", &[]));
        assert!(is_excluded_command("yarn serve", &[]));
        assert!(is_excluded_command("yarn watch", &[]));
        assert!(is_excluded_command("yarn preview", &[]));
        assert!(is_excluded_command("yarn storybook", &[]));
    }

    #[test]
    fn bun_deno_script_runners_are_passthrough() {
        assert!(is_excluded_command("bun run dev", &[]));
        assert!(is_excluded_command("bun run start", &[]));
        assert!(is_excluded_command("bun run serve", &[]));
        assert!(is_excluded_command("bun run watch", &[]));
        assert!(is_excluded_command("bun run preview", &[]));
        assert!(is_excluded_command("bun start", &[]));
        assert!(is_excluded_command("deno task dev", &[]));
        assert!(is_excluded_command("deno task start", &[]));
        assert!(is_excluded_command("deno task serve", &[]));
        assert!(is_excluded_command("deno run --watch main.ts", &[]));
    }

    #[test]
    fn python_servers_are_passthrough() {
        assert!(is_excluded_command("flask run --port 5000", &[]));
        assert!(is_excluded_command("uvicorn app:app --reload", &[]));
        assert!(is_excluded_command("gunicorn app:app -w 4", &[]));
        assert!(is_excluded_command("hypercorn app:app", &[]));
        assert!(is_excluded_command("daphne app.asgi:application", &[]));
        assert!(is_excluded_command(
            "django-admin runserver 0.0.0.0:8000",
            &[]
        ));
        assert!(is_excluded_command("python manage.py runserver", &[]));
        assert!(is_excluded_command("python -m http.server 8080", &[]));
        assert!(is_excluded_command("python3 -m http.server", &[]));
        assert!(is_excluded_command("streamlit run app.py", &[]));
        assert!(is_excluded_command("gradio app.py", &[]));
        assert!(is_excluded_command("celery worker -A app", &[]));
        assert!(is_excluded_command("celery -A app worker", &[]));
        assert!(is_excluded_command("celery -B", &[]));
        assert!(is_excluded_command("dramatiq tasks", &[]));
        assert!(is_excluded_command("rq worker", &[]));
        assert!(is_excluded_command("ptw tests/", &[]));
        assert!(is_excluded_command("pytest-watch", &[]));
    }

    #[test]
    fn ruby_servers_are_passthrough() {
        assert!(is_excluded_command("rails server -p 3000", &[]));
        assert!(is_excluded_command("rails s", &[]));
        assert!(is_excluded_command("puma -C config.rb", &[]));
        assert!(is_excluded_command("unicorn -c config.rb", &[]));
        assert!(is_excluded_command("thin start", &[]));
        assert!(is_excluded_command("foreman start", &[]));
        assert!(is_excluded_command("overmind start", &[]));
        assert!(is_excluded_command("guard -G Guardfile", &[]));
        assert!(is_excluded_command("sidekiq", &[]));
        assert!(is_excluded_command("resque work", &[]));
    }

    #[test]
    fn php_servers_are_passthrough() {
        assert!(is_excluded_command("php artisan serve", &[]));
        assert!(is_excluded_command("php -S localhost:8000", &[]));
        assert!(is_excluded_command("php artisan queue:work", &[]));
        assert!(is_excluded_command("php artisan queue:listen", &[]));
        assert!(is_excluded_command("php artisan horizon", &[]));
        assert!(is_excluded_command("php artisan tinker", &[]));
        assert!(is_excluded_command("sail up", &[]));
    }

    #[test]
    fn java_servers_are_passthrough() {
        assert!(is_excluded_command("./gradlew bootRun", &[]));
        assert!(is_excluded_command("gradlew bootRun", &[]));
        assert!(is_excluded_command("gradle bootRun", &[]));
        assert!(is_excluded_command("mvn spring-boot:run", &[]));
        assert!(is_excluded_command("./mvnw spring-boot:run", &[]));
        assert!(is_excluded_command("mvn quarkus:dev", &[]));
        assert!(is_excluded_command("./mvnw quarkus:dev", &[]));
        assert!(is_excluded_command("sbt run", &[]));
        assert!(is_excluded_command("sbt ~compile", &[]));
        assert!(is_excluded_command("lein run", &[]));
        assert!(is_excluded_command("lein repl", &[]));
        assert!(is_excluded_command("./gradlew run", &[]));
    }

    #[test]
    fn go_servers_are_passthrough() {
        assert!(is_excluded_command("go run main.go", &[]));
        assert!(is_excluded_command("go run ./cmd/server", &[]));
        assert!(is_excluded_command("air -c .air.toml", &[]));
        assert!(is_excluded_command("gin --port 3000", &[]));
        assert!(is_excluded_command("realize start", &[]));
        assert!(is_excluded_command("reflex -r '.go$' go run .", &[]));
        assert!(is_excluded_command("gowatch run", &[]));
    }

    #[test]
    fn dotnet_servers_are_passthrough() {
        assert!(is_excluded_command("dotnet run", &[]));
        assert!(is_excluded_command("dotnet run --project src/Api", &[]));
        assert!(is_excluded_command("dotnet watch run", &[]));
        assert!(is_excluded_command("dotnet ef database update", &[]));
    }

    #[test]
    fn elixir_servers_are_passthrough() {
        assert!(is_excluded_command("mix phx.server", &[]));
        assert!(is_excluded_command("iex -s mix phx.server", &[]));
        assert!(is_excluded_command("iex -S mix phx.server", &[]));
    }

    #[test]
    fn swift_zig_servers_are_passthrough() {
        assert!(is_excluded_command("swift run MyApp", &[]));
        assert!(is_excluded_command("swift package resolve", &[]));
        assert!(is_excluded_command("vapor serve --port 8080", &[]));
        assert!(is_excluded_command("zig build run", &[]));
    }

    #[test]
    fn rust_watchers_are_passthrough() {
        assert!(is_excluded_command("cargo watch -x test", &[]));
        assert!(is_excluded_command("cargo run --bin server", &[]));
        assert!(is_excluded_command("cargo leptos watch", &[]));
        assert!(is_excluded_command("bacon test", &[]));
    }

    #[test]
    fn general_task_runners_are_passthrough() {
        assert!(is_excluded_command("make dev", &[]));
        assert!(is_excluded_command("make serve", &[]));
        assert!(is_excluded_command("make watch", &[]));
        assert!(is_excluded_command("make run", &[]));
        assert!(is_excluded_command("make start", &[]));
        assert!(is_excluded_command("just dev", &[]));
        assert!(is_excluded_command("just serve", &[]));
        assert!(is_excluded_command("just watch", &[]));
        assert!(is_excluded_command("just start", &[]));
        assert!(is_excluded_command("just run", &[]));
        assert!(is_excluded_command("task dev", &[]));
        assert!(is_excluded_command("task serve", &[]));
        assert!(is_excluded_command("task watch", &[]));
        assert!(is_excluded_command("nix develop", &[]));
        assert!(is_excluded_command("devenv up", &[]));
    }

    #[test]
    fn cicd_infra_are_passthrough() {
        assert!(is_excluded_command("act push", &[]));
        assert!(is_excluded_command("docker compose watch", &[]));
        assert!(is_excluded_command("docker-compose watch", &[]));
        assert!(is_excluded_command("skaffold dev", &[]));
        assert!(is_excluded_command("tilt up", &[]));
        assert!(is_excluded_command("garden dev", &[]));
        assert!(is_excluded_command("telepresence connect", &[]));
    }

    #[test]
    fn networking_monitoring_are_passthrough() {
        assert!(is_excluded_command("mtr 8.8.8.8", &[]));
        assert!(is_excluded_command("nmap -sV host", &[]));
        assert!(is_excluded_command("iperf -s", &[]));
        assert!(is_excluded_command("iperf3 -c host", &[]));
        assert!(is_excluded_command("socat TCP-LISTEN:8080,fork -", &[]));
    }

    #[test]
    fn load_testing_is_passthrough() {
        assert!(is_excluded_command("ab -n 1000 http://localhost/", &[]));
        assert!(is_excluded_command("wrk -t12 -c400 http://localhost/", &[]));
        assert!(is_excluded_command("hey -n 10000 http://localhost/", &[]));
        assert!(is_excluded_command("vegeta attack", &[]));
        assert!(is_excluded_command("k6 run script.js", &[]));
        assert!(is_excluded_command("artillery run test.yml", &[]));
    }

    #[test]
    fn smart_script_detection_works() {
        assert!(is_excluded_command("npm run dev:ssr", &[]));
        assert!(is_excluded_command("npm run dev:local", &[]));
        assert!(is_excluded_command("yarn start:production", &[]));
        assert!(is_excluded_command("pnpm run serve:local", &[]));
        assert!(is_excluded_command("bun run watch:css", &[]));
        assert!(is_excluded_command("deno task dev:api", &[]));
        assert!(is_excluded_command("npm run storybook:ci", &[]));
        assert!(is_excluded_command("yarn preview:staging", &[]));
        assert!(is_excluded_command("pnpm run hot-reload", &[]));
        assert!(is_excluded_command("npm run hmr-server", &[]));
        assert!(is_excluded_command("bun run live-server", &[]));
    }

    #[test]
    fn smart_detection_does_not_false_positive() {
        assert!(!is_excluded_command("npm run build", &[]));
        assert!(!is_excluded_command("npm run lint", &[]));
        assert!(!is_excluded_command("npm run test", &[]));
        assert!(!is_excluded_command("npm run format", &[]));
        assert!(!is_excluded_command("yarn build", &[]));
        assert!(!is_excluded_command("yarn test", &[]));
        assert!(!is_excluded_command("pnpm run lint", &[]));
        assert!(!is_excluded_command("bun run build", &[]));
    }

    #[test]
    fn gh_fully_excluded() {
        assert!(is_excluded_command("gh", &[]));
        assert!(is_excluded_command(
            "gh pr close --comment 'closing — see #407'",
            &[]
        ));
        assert!(is_excluded_command(
            "gh issue create --title \"bug\" --body \"desc\"",
            &[]
        ));
        assert!(is_excluded_command("gh api repos/owner/repo/pulls", &[]));
        assert!(is_excluded_command("gh run list --limit 5", &[]));
    }
}

#[cfg(test)]
mod verbatim_output_tests {
    use super::{compress_if_beneficial, is_verbatim_output};

    #[test]
    fn http_clients_are_verbatim() {
        assert!(is_verbatim_output("curl https://api.example.com"));
        assert!(is_verbatim_output(
            "curl -s -H 'Accept: application/json' https://api.example.com/data"
        ));
        assert!(is_verbatim_output(
            "curl -X POST -d '{\"key\":\"val\"}' https://api.example.com"
        ));
        assert!(is_verbatim_output("/usr/bin/curl https://example.com"));
        assert!(is_verbatim_output("wget -qO- https://example.com"));
        assert!(is_verbatim_output("wget https://example.com/file.json"));
        assert!(is_verbatim_output("http GET https://api.example.com"));
        assert!(is_verbatim_output("https PUT https://api.example.com/data"));
        assert!(is_verbatim_output("xh https://api.example.com"));
        assert!(is_verbatim_output("curlie https://api.example.com"));
        assert!(is_verbatim_output(
            "grpcurl -plaintext localhost:50051 list"
        ));
    }

    #[test]
    fn file_viewers_are_verbatim() {
        assert!(is_verbatim_output("cat package.json"));
        assert!(is_verbatim_output("cat /etc/hosts"));
        assert!(is_verbatim_output("/bin/cat file.txt"));
        assert!(is_verbatim_output("bat src/main.rs"));
        assert!(is_verbatim_output("batcat README.md"));
        assert!(is_verbatim_output("head -20 log.txt"));
        assert!(is_verbatim_output("head -n 50 file.rs"));
        assert!(is_verbatim_output("tail -100 server.log"));
        assert!(is_verbatim_output("tail -n 20 file.txt"));
    }

    #[test]
    fn tail_follow_not_verbatim() {
        assert!(!is_verbatim_output("tail -f /var/log/syslog"));
        assert!(!is_verbatim_output("tail --follow server.log"));
    }

    #[test]
    fn data_format_tools_are_verbatim() {
        assert!(is_verbatim_output("jq '.items' data.json"));
        assert!(is_verbatim_output("jq -r '.name' package.json"));
        assert!(is_verbatim_output("yq '.spec' deployment.yaml"));
        assert!(is_verbatim_output("xq '.rss.channel.title' feed.xml"));
        assert!(is_verbatim_output("fx data.json"));
        assert!(is_verbatim_output("gron data.json"));
        assert!(is_verbatim_output("mlr --csv head -n 5 data.csv"));
        assert!(is_verbatim_output("miller --json head data.json"));
        assert!(is_verbatim_output("dasel -f config.toml '.database.host'"));
        assert!(is_verbatim_output("csvlook data.csv"));
        assert!(is_verbatim_output("csvcut -c 1,3 data.csv"));
        assert!(is_verbatim_output("csvjson data.csv"));
    }

    #[test]
    fn binary_viewers_are_verbatim() {
        assert!(is_verbatim_output("xxd binary.dat"));
        assert!(is_verbatim_output("hexdump -C binary.dat"));
        assert!(is_verbatim_output("od -A x -t x1z binary.dat"));
        assert!(is_verbatim_output("strings /usr/bin/curl"));
        assert!(is_verbatim_output("file unknown.bin"));
    }

    #[test]
    fn infra_inspection_is_verbatim() {
        assert!(is_verbatim_output("terraform output"));
        assert!(is_verbatim_output("terraform show"));
        assert!(is_verbatim_output("terraform state show aws_instance.web"));
        assert!(is_verbatim_output("terraform state list"));
        assert!(is_verbatim_output("terraform state pull"));
        assert!(is_verbatim_output("tofu output"));
        assert!(is_verbatim_output("tofu show"));
        assert!(is_verbatim_output("pulumi stack output"));
        assert!(is_verbatim_output("pulumi stack export"));
        assert!(is_verbatim_output("docker inspect my-container"));
        assert!(is_verbatim_output("podman inspect my-pod"));
        assert!(is_verbatim_output("kubectl get pods -o yaml"));
        assert!(is_verbatim_output("kubectl get deploy -ojson"));
        assert!(is_verbatim_output("kubectl get svc --output yaml"));
        assert!(is_verbatim_output("kubectl get pods --output=json"));
        assert!(is_verbatim_output("k get pods -o yaml"));
        assert!(is_verbatim_output("kubectl describe pod my-pod"));
        assert!(is_verbatim_output("k describe deployment web"));
        assert!(is_verbatim_output("helm get values my-release"));
        assert!(is_verbatim_output("helm template my-chart"));
    }

    #[test]
    fn terraform_plan_not_verbatim() {
        assert!(!is_verbatim_output("terraform plan"));
        assert!(!is_verbatim_output("terraform apply"));
        assert!(!is_verbatim_output("terraform init"));
    }

    #[test]
    fn kubectl_get_is_now_verbatim() {
        assert!(is_verbatim_output("kubectl get pods"));
        assert!(is_verbatim_output("kubectl get deployments"));
    }

    #[test]
    fn crypto_commands_are_verbatim() {
        assert!(is_verbatim_output("openssl x509 -in cert.pem -text"));
        assert!(is_verbatim_output(
            "openssl s_client -connect example.com:443"
        ));
        assert!(is_verbatim_output("openssl req -new -x509 -key key.pem"));
        assert!(is_verbatim_output("gpg --list-keys"));
        assert!(is_verbatim_output("ssh-keygen -l -f key.pub"));
    }

    #[test]
    fn database_queries_are_verbatim() {
        assert!(is_verbatim_output(r#"psql -c "SELECT * FROM users" mydb"#));
        assert!(is_verbatim_output("psql --command 'SELECT 1' mydb"));
        assert!(is_verbatim_output(r#"mysql -e "SELECT * FROM users" mydb"#));
        assert!(is_verbatim_output("mysql --execute 'SHOW TABLES' mydb"));
        assert!(is_verbatim_output(
            r#"mariadb -e "SELECT * FROM users" mydb"#
        ));
        assert!(is_verbatim_output(
            r#"sqlite3 data.db "SELECT * FROM users""#
        ));
        assert!(is_verbatim_output("mongosh --eval 'db.users.find()' mydb"));
    }

    #[test]
    fn interactive_db_not_verbatim() {
        assert!(!is_verbatim_output("psql mydb"));
        assert!(!is_verbatim_output("mysql -u root mydb"));
    }

    #[test]
    fn dns_network_inspection_is_verbatim() {
        assert!(is_verbatim_output("dig example.com"));
        assert!(is_verbatim_output("dig +short example.com A"));
        assert!(is_verbatim_output("nslookup example.com"));
        assert!(is_verbatim_output("host example.com"));
        assert!(is_verbatim_output("whois example.com"));
        assert!(is_verbatim_output("drill example.com"));
    }

    #[test]
    fn language_one_liners_are_verbatim() {
        assert!(is_verbatim_output(
            "python -c 'import json; print(json.dumps({\"key\": \"value\"}))'"
        ));
        assert!(is_verbatim_output("python3 -c 'print(42)'"));
        assert!(is_verbatim_output(
            "node -e 'console.log(JSON.stringify({a:1}))'"
        ));
        assert!(is_verbatim_output("node --eval 'console.log(1)'"));
        assert!(is_verbatim_output("ruby -e 'puts 42'"));
        assert!(is_verbatim_output("perl -e 'print 42'"));
        assert!(is_verbatim_output("php -r 'echo json_encode([1,2,3]);'"));
    }

    #[test]
    fn language_scripts_not_verbatim() {
        assert!(!is_verbatim_output("python script.py"));
        assert!(!is_verbatim_output("node server.js"));
        assert!(!is_verbatim_output("ruby app.rb"));
    }

    #[test]
    fn container_listings_are_verbatim() {
        assert!(is_verbatim_output("docker ps"));
        assert!(is_verbatim_output("docker ps -a"));
        assert!(is_verbatim_output("docker images"));
        assert!(is_verbatim_output("docker images -a"));
        assert!(is_verbatim_output("podman ps"));
        assert!(is_verbatim_output("podman images"));
        assert!(is_verbatim_output("kubectl get pods"));
        assert!(is_verbatim_output("kubectl get deployments -A"));
        assert!(is_verbatim_output("kubectl get svc --all-namespaces"));
        assert!(is_verbatim_output("k get pods"));
        assert!(is_verbatim_output("helm list"));
        assert!(is_verbatim_output("helm ls --all-namespaces"));
        assert!(is_verbatim_output("docker compose ps"));
        assert!(is_verbatim_output("docker-compose ps"));
    }

    #[test]
    fn file_listings_are_verbatim() {
        assert!(is_verbatim_output("find . -name '*.rs'"));
        assert!(is_verbatim_output("find /var/log -type f"));
        assert!(is_verbatim_output("fd --extension rs"));
        assert!(is_verbatim_output("fdfind .rs src/"));
        assert!(is_verbatim_output("ls -la"));
        assert!(is_verbatim_output("ls -lah /tmp"));
        assert!(is_verbatim_output("exa -la"));
        assert!(is_verbatim_output("eza --long"));
    }

    #[test]
    fn system_queries_are_verbatim() {
        assert!(is_verbatim_output("stat file.txt"));
        assert!(is_verbatim_output("wc -l file.txt"));
        assert!(is_verbatim_output("du -sh /var"));
        assert!(is_verbatim_output("df -h"));
        assert!(is_verbatim_output("free -m"));
        assert!(is_verbatim_output("uname -a"));
        assert!(is_verbatim_output("id"));
        assert!(is_verbatim_output("whoami"));
        assert!(is_verbatim_output("hostname"));
        assert!(is_verbatim_output("which python3"));
        assert!(is_verbatim_output("readlink -f ./link"));
        assert!(is_verbatim_output("sha256sum file.tar.gz"));
        assert!(is_verbatim_output("base64 file.bin"));
        assert!(is_verbatim_output("ip addr show"));
        assert!(is_verbatim_output("ss -tlnp"));
    }

    #[test]
    fn pipe_tail_detection() {
        assert!(
            is_verbatim_output("kubectl get pods -o json | jq '.items[].metadata.name'"),
            "piped to jq must be verbatim"
        );
        assert!(
            is_verbatim_output("aws s3api list-objects --bucket x | jq '.Contents'"),
            "piped to jq must be verbatim"
        );
        assert!(
            is_verbatim_output("docker inspect web | head -50"),
            "piped to head must be verbatim"
        );
        assert!(
            is_verbatim_output("terraform state pull | jq '.resources'"),
            "piped to jq must be verbatim"
        );
        assert!(
            is_verbatim_output("echo hello | wc -l"),
            "piped to wc (system query) should be verbatim"
        );
    }

    #[test]
    fn build_commands_not_verbatim() {
        assert!(!is_verbatim_output("cargo build"));
        assert!(!is_verbatim_output("npm run build"));
        assert!(!is_verbatim_output("make"));
        assert!(!is_verbatim_output("docker build ."));
        assert!(!is_verbatim_output("go build ./..."));
        assert!(!is_verbatim_output("cargo test"));
        assert!(!is_verbatim_output("pytest"));
        assert!(!is_verbatim_output("npm install"));
        assert!(!is_verbatim_output("pip install requests"));
        assert!(!is_verbatim_output("terraform plan"));
        assert!(!is_verbatim_output("terraform apply"));
    }

    #[test]
    fn cloud_cli_queries_are_verbatim() {
        assert!(is_verbatim_output("aws sts get-caller-identity"));
        assert!(is_verbatim_output("aws ec2 describe-instances"));
        assert!(is_verbatim_output(
            "aws s3api list-objects --bucket my-bucket"
        ));
        assert!(is_verbatim_output("aws iam list-users"));
        assert!(is_verbatim_output("aws ecs describe-tasks --cluster x"));
        assert!(is_verbatim_output("aws rds describe-db-instances"));
        assert!(is_verbatim_output("gcloud compute instances list"));
        assert!(is_verbatim_output("gcloud projects describe my-project"));
        assert!(is_verbatim_output("gcloud iam roles list"));
        assert!(is_verbatim_output("gcloud container clusters list"));
        assert!(is_verbatim_output("az vm list"));
        assert!(is_verbatim_output("az account show"));
        assert!(is_verbatim_output("az network nsg list"));
        assert!(is_verbatim_output("az aks show --name mycluster"));
    }

    #[test]
    fn cloud_cli_mutations_not_verbatim() {
        assert!(!is_verbatim_output("aws configure"));
        assert!(!is_verbatim_output("gcloud auth login"));
        assert!(!is_verbatim_output("az login"));
        assert!(!is_verbatim_output("gcloud app deploy"));
    }

    #[test]
    fn package_manager_info_is_verbatim() {
        assert!(is_verbatim_output("npm list"));
        assert!(is_verbatim_output("npm ls --all"));
        assert!(is_verbatim_output("npm info react"));
        assert!(is_verbatim_output("npm view react versions"));
        assert!(is_verbatim_output("npm outdated"));
        assert!(is_verbatim_output("npm audit"));
        assert!(is_verbatim_output("yarn list"));
        assert!(is_verbatim_output("yarn info react"));
        assert!(is_verbatim_output("yarn why react"));
        assert!(is_verbatim_output("yarn audit"));
        assert!(is_verbatim_output("pnpm list"));
        assert!(is_verbatim_output("pnpm why react"));
        assert!(is_verbatim_output("pnpm outdated"));
        assert!(is_verbatim_output("pip list"));
        assert!(is_verbatim_output("pip show requests"));
        assert!(is_verbatim_output("pip freeze"));
        assert!(is_verbatim_output("pip3 list"));
        assert!(is_verbatim_output("gem list"));
        assert!(is_verbatim_output("gem info rails"));
        assert!(is_verbatim_output("cargo metadata"));
        assert!(is_verbatim_output("cargo tree"));
        assert!(is_verbatim_output("go list ./..."));
        assert!(is_verbatim_output("go version"));
        assert!(is_verbatim_output("composer show"));
        assert!(is_verbatim_output("composer outdated"));
        assert!(is_verbatim_output("brew list"));
        assert!(is_verbatim_output("brew info node"));
        assert!(is_verbatim_output("brew deps node"));
        assert!(is_verbatim_output("apt list --installed"));
        assert!(is_verbatim_output("apt show nginx"));
        assert!(is_verbatim_output("dpkg -l"));
        assert!(is_verbatim_output("dpkg -s nginx"));
    }

    #[test]
    fn package_manager_install_not_verbatim() {
        assert!(!is_verbatim_output("npm install"));
        assert!(!is_verbatim_output("yarn add react"));
        assert!(!is_verbatim_output("pip install requests"));
        assert!(!is_verbatim_output("cargo build"));
        assert!(!is_verbatim_output("go build"));
        assert!(!is_verbatim_output("brew install node"));
        assert!(!is_verbatim_output("apt install nginx"));
    }

    #[test]
    fn version_and_help_are_verbatim() {
        assert!(is_verbatim_output("node --version"));
        assert!(is_verbatim_output("python3 --version"));
        assert!(is_verbatim_output("rustc -V"));
        assert!(is_verbatim_output("docker version"));
        assert!(is_verbatim_output("git --version"));
        assert!(is_verbatim_output("cargo --help"));
        assert!(is_verbatim_output("docker help"));
        assert!(is_verbatim_output("git -h"));
        assert!(is_verbatim_output("npm help install"));
    }

    #[test]
    fn version_flag_needs_binary_context() {
        assert!(!is_verbatim_output("--version"));
        assert!(
            !is_verbatim_output("some command with --version and other args too"),
            "commands with 4+ tokens should not match version check"
        );
    }

    #[test]
    fn config_viewers_are_verbatim() {
        assert!(is_verbatim_output("git config --list"));
        assert!(is_verbatim_output("git config --global --list"));
        assert!(is_verbatim_output("git config user.email"));
        assert!(is_verbatim_output("npm config list"));
        assert!(is_verbatim_output("npm config get registry"));
        assert!(is_verbatim_output("yarn config list"));
        assert!(is_verbatim_output("pip config list"));
        assert!(is_verbatim_output("rustup show"));
        assert!(is_verbatim_output("rustup target list"));
        assert!(is_verbatim_output("docker context ls"));
        assert!(is_verbatim_output("kubectl config view"));
        assert!(is_verbatim_output("kubectl config get-contexts"));
        assert!(is_verbatim_output("kubectl config current-context"));
    }

    #[test]
    fn config_setters_not_verbatim() {
        assert!(!is_verbatim_output("git config --set user.name foo"));
        assert!(!is_verbatim_output("git config --unset user.name"));
    }

    #[test]
    fn log_viewers_are_verbatim() {
        assert!(is_verbatim_output("journalctl -u nginx"));
        assert!(is_verbatim_output("journalctl --since '1 hour ago'"));
        assert!(is_verbatim_output("dmesg"));
        assert!(is_verbatim_output("dmesg --level=err"));
        assert!(is_verbatim_output("docker logs mycontainer"));
        assert!(is_verbatim_output("docker logs --tail 100 web"));
        assert!(is_verbatim_output("kubectl logs pod/web"));
        assert!(is_verbatim_output("docker compose logs web"));
    }

    #[test]
    fn follow_logs_not_verbatim() {
        assert!(!is_verbatim_output("journalctl -f"));
        assert!(!is_verbatim_output("journalctl --follow -u nginx"));
        assert!(!is_verbatim_output("dmesg -w"));
        assert!(!is_verbatim_output("dmesg --follow"));
        assert!(!is_verbatim_output("docker logs -f web"));
        assert!(!is_verbatim_output("kubectl logs -f pod/web"));
        assert!(!is_verbatim_output("docker compose logs -f"));
    }

    #[test]
    fn archive_listings_are_verbatim() {
        assert!(is_verbatim_output("tar -tf archive.tar.gz"));
        assert!(is_verbatim_output("tar tf archive.tar"));
        assert!(is_verbatim_output("unzip -l archive.zip"));
        assert!(is_verbatim_output("zipinfo archive.zip"));
        assert!(is_verbatim_output("lsar archive.7z"));
    }

    #[test]
    fn clipboard_tools_are_verbatim() {
        assert!(is_verbatim_output("pbpaste"));
        assert!(is_verbatim_output("wl-paste"));
        assert!(is_verbatim_output("xclip -o"));
        assert!(is_verbatim_output("xclip -selection clipboard -o"));
        assert!(is_verbatim_output("xsel -o"));
        assert!(is_verbatim_output("xsel --output"));
    }

    #[test]
    fn git_data_commands_are_verbatim() {
        assert!(is_verbatim_output("git remote -v"));
        assert!(is_verbatim_output("git remote show origin"));
        assert!(is_verbatim_output("git config --list"));
        assert!(is_verbatim_output("git rev-parse HEAD"));
        assert!(is_verbatim_output("git rev-parse --show-toplevel"));
        assert!(is_verbatim_output("git ls-files"));
        assert!(is_verbatim_output("git ls-tree HEAD"));
        assert!(is_verbatim_output("git ls-remote origin"));
        assert!(is_verbatim_output("git shortlog -sn"));
        assert!(is_verbatim_output("git for-each-ref --format='%(refname)'"));
        assert!(is_verbatim_output("git cat-file -p HEAD"));
        assert!(is_verbatim_output("git describe --tags"));
        assert!(is_verbatim_output("git merge-base main feature"));
    }

    #[test]
    fn git_mutations_not_verbatim_via_git_data() {
        assert!(!super::is_git_data_command("git commit -m 'fix'"));
        assert!(!super::is_git_data_command("git push"));
        assert!(!super::is_git_data_command("git pull"));
        assert!(!super::is_git_data_command("git fetch"));
        assert!(!super::is_git_data_command("git add ."));
        assert!(!super::is_git_data_command("git rebase main"));
        assert!(!super::is_git_data_command("git cherry-pick abc123"));
    }

    #[test]
    fn task_dry_run_is_verbatim() {
        assert!(is_verbatim_output("make -n build"));
        assert!(is_verbatim_output("make --dry-run"));
        assert!(is_verbatim_output("ansible-playbook --check site.yml"));
        assert!(is_verbatim_output(
            "ansible-playbook --diff --check site.yml"
        ));
    }

    #[test]
    fn task_execution_not_verbatim() {
        assert!(!is_verbatim_output("make build"));
        assert!(!is_verbatim_output("make"));
        assert!(!is_verbatim_output("ansible-playbook site.yml"));
    }

    #[test]
    fn env_dump_is_verbatim() {
        assert!(is_verbatim_output("env"));
        assert!(is_verbatim_output("printenv"));
        assert!(is_verbatim_output("printenv PATH"));
        assert!(is_verbatim_output("locale"));
    }

    #[test]
    fn curl_json_output_preserved() {
        let json = r#"{"users":[{"id":1,"name":"Alice","email":"alice@example.com"},{"id":2,"name":"Bob","email":"bob@example.com"}],"total":2,"page":1}"#;
        let result = compress_if_beneficial("curl https://api.example.com/users", json);
        assert!(
            result.contains("alice@example.com"),
            "curl JSON data must be preserved verbatim, got: {result}"
        );
        assert!(
            result.contains(r#""name":"Bob""#),
            "curl JSON data must be preserved verbatim, got: {result}"
        );
    }

    #[test]
    fn curl_html_output_preserved() {
        let html = "<!DOCTYPE html><html><head><title>Test Page</title></head><body><h1>Hello World</h1><p>Some important content here that should not be summarized.</p></body></html>";
        let result = compress_if_beneficial("curl https://example.com", html);
        assert!(
            result.contains("Hello World"),
            "curl HTML content must be preserved, got: {result}"
        );
        assert!(
            result.contains("important content"),
            "curl HTML content must be preserved, got: {result}"
        );
    }

    #[test]
    fn curl_headers_preserved() {
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Request-Id: abc-123\r\nX-RateLimit-Remaining: 59\r\nContent-Length: 1234\r\nServer: nginx\r\nDate: Mon, 01 Jan 2024 00:00:00 GMT\r\n\r\n";
        let result = compress_if_beneficial("curl -I https://api.example.com", headers);
        assert!(
            result.contains("X-Request-Id: abc-123"),
            "curl headers must be preserved, got: {result}"
        );
        assert!(
            result.contains("X-RateLimit-Remaining"),
            "curl headers must be preserved, got: {result}"
        );
    }

    #[test]
    fn cat_output_preserved() {
        let content = r#"{
  "name": "lean-ctx",
  "version": "3.5.16",
  "description": "Context Runtime for AI Agents",
  "main": "index.js",
  "scripts": {
    "build": "cargo build --release",
    "test": "cargo test"
  }
}"#;
        let result = compress_if_beneficial("cat package.json", content);
        assert!(
            result.contains(r#""version": "3.5.16""#),
            "cat output must be preserved, got: {result}"
        );
    }

    #[test]
    fn jq_output_preserved() {
        let json = r#"[
  {"id": 1, "status": "active", "name": "Alice"},
  {"id": 2, "status": "inactive", "name": "Bob"},
  {"id": 3, "status": "active", "name": "Charlie"}
]"#;
        let result =
            compress_if_beneficial("jq '.[] | select(.status==\"active\")' data.json", json);
        assert!(
            result.contains("Charlie"),
            "jq output must be preserved, got: {result}"
        );
    }

    #[test]
    fn wget_output_preserved() {
        let content = r#"{"key": "value", "data": [1, 2, 3]}"#;
        let result = compress_if_beneficial("wget -qO- https://api.example.com/data", content);
        assert!(
            result.contains(r#""data": [1, 2, 3]"#),
            "wget data output must be preserved, got: {result}"
        );
    }

    #[test]
    fn large_curl_output_gets_truncated_not_destroyed() {
        let mut json = String::from("[");
        for i in 0..500 {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                r#"{{"id":{i},"name":"user_{i}","email":"user{i}@example.com","role":"admin"}}"#
            ));
        }
        json.push(']');
        let result = compress_if_beneficial("curl https://api.example.com/all-users", &json);
        assert!(
            result.contains("user_0"),
            "first items must be preserved in truncated output, got len: {}",
            result.len()
        );
        if result.contains("lines omitted") {
            assert!(
                result.contains("verbatim truncated"),
                "must mark as verbatim truncated, got: {result}"
            );
        }
    }
}

#[cfg(test)]
mod cli_api_data_tests {
    use super::is_verbatim_output;

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
}

#[cfg(test)]
mod structural_output_tests {
    use super::has_structural_output;

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
        assert!(!has_structural_output("git log --stat"));
        assert!(!has_structural_output("git log -n 5"));
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
        assert!(!has_structural_output("git commit -m 'fix'"));
        assert!(!has_structural_output("git push"));
        assert!(!has_structural_output("git pull"));
        assert!(!has_structural_output("git branch"));
        assert!(!has_structural_output("git fetch"));
        assert!(!has_structural_output("git add ."));
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
        let result = super::compress_if_beneficial("git diff", diff);
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
        let result = super::compress_if_beneficial("git diff", &diff);
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
}
