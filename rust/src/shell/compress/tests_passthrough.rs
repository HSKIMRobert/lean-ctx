use super::*;
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
    assert!(!is_excluded_command("gh pr close --comment 'done'", &[]));
    assert!(!is_excluded_command("gh issue list", &[]));
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
fn gh_auth_excluded_but_data_commands_not() {
    assert!(is_excluded_command("gh auth login", &[]));
    assert!(is_excluded_command("gh browse", &[]));
    assert!(!is_excluded_command("gh pr list", &[]));
    assert!(!is_excluded_command("gh issue list", &[]));
    assert!(!is_excluded_command("gh api repos/owner/repo/pulls", &[]));
    assert!(!is_excluded_command("gh run list --limit 5", &[]));
}
