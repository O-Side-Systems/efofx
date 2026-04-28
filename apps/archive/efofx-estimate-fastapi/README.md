
# 📐 efOfX Estimation Service (efofx-estimate)

The **efOfX Estimation Service** is the core monolithic backend responsible for providing natural language-driven project estimation using Reference Class Forecasting (RCF). It is the intelligent engine behind the efOfX platform, offering multitenant support, structured estimation logic, chat-based iterative project scoping, and integration with OpenAI or other LLM providers.

---

## 🧠 Responsibilities

The `efofx-estimate` service performs the following core functions:

1. **Conversational Estimation Flow**
   - Iterative chat with user to define project scope
   - Maintains chat context per session
   - Gathers necessary project details until user confirmation

2. **Reference Class Forecasting (RCF)**
   - Classifies the project using MCP-aligned reference classes
   - Retrieves region-aware reference project data
   - Computes structured estimates (cost, time, team size)
   - Applies cost breakdown by class-defined categories
   - Adjusts estimates using tuning factors (by class + region)

3. **LLM Integration**
   - Constructs structured prompts for OpenAI (or other LLMs)
   - Injects context and reference data using pre-defined templates
   - Handles LLM responses (structured + natural language)

4. **Multitenancy**
   - Supports tenant-specific OpenAI API keys and configuration
   - Isolates all data and requests by tenant ID

5. **Persistence**
   - Saves completed estimates to MongoDB
   - Stores user chat sessions, feedback, and photos
   - Prepares feedback data for batch tuning

6. **Image Support**
   - Accepts optional photo uploads per project session

---

## 🗂 Directory Structure

Follows best practices for scalable, testable Python projects that are easy to decompose into microservices later.

```
efofx-estimate/
├── app/
│   ├── api/                # FastAPI route handlers
│   ├── core/               # Config, logging, security, constants
│   ├── models/             # Pydantic and DB schemas
│   ├── services/           # Business logic (estimation, LLM, chat)
│   ├── db/                 # MongoDB access layer
│   ├── utils/              # Helper functions
│   └── main.py             # FastAPI entrypoint
├── tests/                  # Pytest unit/integration tests
│   ├── fixtures/
│   ├── api/
│   └── services/
├── scripts/                # DB migrations, tuning jobs, LLM prompts
├── .env                    # Environment variables (example)
├── requirements.txt        # Dependency versions
├── README.md               # This file
└── pyproject.toml          # Build + linting tools
```

---

## 🧪 Testability

- All business logic is isolated in `services/` and injected via dependency overrides
- Uses **Pytest** for unit and integration tests
- Mockable LLM and DB interfaces
- Clean separation of:
  - Input validation (Pydantic)
  - Business rules (services)
  - I/O (api/db layers)

---

## 💾 Database

- **Type**: MongoDB (document-oriented)
- **Collections**:
  - `tenants`
  - `reference_classes`
  - `reference_projects`
  - `estimates`
  - `feedback`

---

## ✨ Code Style & Linting

- Follows [Google’s Python Style Guide](https://google.github.io/styleguide/pyguide.html)
- Enforced via:
  - `black` for formatting
  - `flake8` for linting
  - `mypy` for static typing
- All modules, functions, and classes include docstrings in [Google docstring style](https://google.github.io/styleguide/pyguide.html#38-comments-and-docstrings)

---

## 📦 Dependencies

> *(To be filled in during implementation)*

| Library       | Version | Purpose                |
| ------------- | ------- | ---------------------- |
| FastAPI       | 0.116.1 | Web framework          |
| PyMongo Async | 4.13.2  | Async MongoDB driver   |
| Pydantic      | 2.11.7  | Data validation        |
| OpenAI        | TBD     | LLM API integration    |
| python-dotenv | 1.1.1   | Environment management |
| Pytest        | 8.4.1   | Testing framework      |


---

## 🚀 Deployment

- **Host**: DigitalOcean
- **Infra**: Droplet or App Platform
- **CI/CD**: GitHub Actions
- **ENV Variables**:
  - `MONGO_URI`
  - `OPENAI_API_KEY` (or tenant-provided)


---

## 🛡️ Security

- Tenant-based API key scoping
- All APIs require authentication (key or JWT-based)
- Audit logs and error tracking (optional via Sentry or custom logging)

---

## 📎 Related Projects

- `efofx-core`: Domain logic and models (optional future lib)
- `efofx-admin`: Internal dashboards for feedback, tuning, and analytics
- `efofx-ui`: Frontend or integration-ready widgets (future)

---

## 👀 Example Usage

```bash
curl -X POST https://api.efOfX.ai/estimate/start \
  -H "Authorization: Bearer <TENANT_API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
        "description": "I want to install a 15x30 foot pool with spa in my backyard.",
        "region": "SoCal - Coastal"
      }'
```

---

> _Built with math, muscle, and machine learning._ 🧠
