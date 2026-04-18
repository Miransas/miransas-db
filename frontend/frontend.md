miransas-db/
├─ .config/
├─ .github/
│  └─ workflows/
├─ backend/
│  ├─ Cargo.toml
│  ├─ README.md
│  ├─ .env.example
│  ├─ migrations/
│  ├─ scripts/
│  ├─ config/
│  │  ├─ development.toml
│  │  ├─ production.toml
│  │  └─ test.toml
│  ├─ docs/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ lib.rs
│  │  ├─ app/
│  │  │  ├─ mod.rs
│  │  │  ├─ state.rs
│  │  │  └─ bootstrap.rs
│  │  ├─ config/
│  │  │  ├─ mod.rs
│  │  │  └─ env.rs
│  │  ├─ server/
│  │  │  ├─ mod.rs
│  │  │  ├─ router.rs
│  │  │  └─ middleware.rs
│  │  ├─ api/
│  │  │  ├─ mod.rs
│  │  │  ├─ health.rs
│  │  │  ├─ auth.rs
│  │  │  ├─ users.rs
│  │  │  ├─ databases.rs
│  │  │  └─ admin.rs
│  │  ├─ domain/
│  │  │  ├─ mod.rs
│  │  │  ├─ user.rs
│  │  │  ├─ database.rs
│  │  │  ├─ project.rs
│  │  │  └─ token.rs
│  │  ├─ services/
│  │  │  ├─ mod.rs
│  │  │  ├─ auth_service.rs
│  │  │  ├─ database_service.rs
│  │  │  └─ provisioning_service.rs
│  │  ├─ db/
│  │  │  ├─ mod.rs
│  │  │  ├─ pool.rs
│  │  │  ├─ schema.rs
│  │  │  ├─ repositories/
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ user_repo.rs
│  │  │  │  └─ database_repo.rs
│  │  │  └─ models/
│  │  │     ├─ mod.rs
│  │  │     ├─ user_model.rs
│  │  │     └─ database_model.rs
│  │  ├─ errors/
│  │  │  ├─ mod.rs
│  │  │  └─ app_error.rs
│  │  ├─ telemetry/
│  │  │  ├─ mod.rs
│  │  │  ├─ logging.rs
│  │  │  └─ tracing.rs
│  │  └─ utils/
│  │     ├─ mod.rs
│  │     └─ time.rs
│  └─ tests/
│     ├─ api_health.rs
│     ├─ auth_flow.rs
│     └─ database_flow.rs
├─ benchmarks/
├─ changeset/
├─ devcontainer/
├─ docker/
│  ├─ Dockerfile.backend
│  ├─ docker-compose.dev.yml
│  └─ docker-compose.prod.yml
├─ frontend/
└─ .gitignore