# Non-Functional Requirements

## Performance

**NFR-P1: API Response Times**
- FastAPI endpoints p95 < 500ms (excluding LLM calls)
- LLM-powered endpoints p95 < 3 seconds
- MCP functions p95 < 150ms (warm), cold start < 300ms
- Database queries p95 < 50ms
- **Rationale:** User-facing estimation requests must feel responsive, especially in widget context

**NFR-P2: Throughput & Concurrency**
- Support 100 concurrent estimation requests per tenant (Pro tier)
- Support 1000 concurrent estimation requests across platform
- Widget sessions: 50 concurrent conversations per tenant
- **Rationale:** Contractors may embed widget on high-traffic sites

**NFR-P3: LLM Performance**
- OpenAI API calls complete within 2-5 seconds
- Retry failed LLM calls with exponential backoff (max 3 retries)
- Cache LLM responses for identical inputs (1 hour TTL)
- **Rationale:** LLM calls are the slowest component, must be optimized

**NFR-P4: Database Performance**
- MongoDB connection pool: 10-50 connections
- Index coverage for all tenant-scoped queries
- Compound indexes on (tenant_id, category, region)
- Query plan analysis for slow queries (>100ms)
- **Rationale:** Multi-tenant queries must be fast despite large datasets

## Security

**NFR-S1: Data Encryption**
- All data encrypted in transit (TLS 1.3)
- Tenant OpenAI keys encrypted at rest (AES-256)
- Passwords hashed with bcrypt (cost factor 12)
- API keys hashed for storage, plaintext never retrievable
- **Compliance:** Required for SOC 2 compliance (future)

**NFR-S2: Authentication & Authorization**
- JWT tokens expire after 24 hours
- Refresh tokens expire after 30 days
- Rate limit failed login attempts (5 per 15 minutes)
- MFA support for Enterprise tier tenants
- **Rationale:** Prevent unauthorized access and brute force attacks

**NFR-S3: Tenant Isolation (Zero Trust)**
- 100% of database queries include tenant_id filter
- API endpoints validate tenant ownership before operations
- No shared resources between tenants (reference classes excepted)
- Security audit confirms zero cross-tenant data leakage
- **Compliance:** Critical for multi-tenant SaaS trust

**NFR-S4: API Security**
- CORS configured for known origins only
- CSRF protection on state-changing operations
- Input sanitization prevents SQL/NoSQL injection
- Rate limiting prevents DDoS (per-tenant and global)
- **Rationale:** Publicly accessible API must resist attacks

**NFR-S5: Widget Security**
- Widget API key never exposed in client code (proxy through session tokens)
- XSS protection via Content Security Policy (CSP)
- Session tokens expire after 24 hours
- HTTPS required for all widget communications
- **Rationale:** Widget is end-customer facing, highest attack surface

## Scalability

**NFR-SC1: Horizontal Scaling**
- FastAPI backend stateless, scales horizontally behind load balancer
- MCP functions auto-scale based on demand (serverless)
- MongoDB Atlas auto-scaling for storage and compute
- No session affinity required
- **Rationale:** Must handle tenant growth without architecture changes

**NFR-SC2: Data Growth**
- Support 100,000 estimates per month across all tenants
- Support 10,000 reference classes (platform + custom)
- Support 50,000 feedback submissions per month
- MongoDB sharding strategy planned for >1TB data
- **Rationale:** Growth projections for year 1

**NFR-SC3: Multi-Region Support (Future)**
- Architecture supports multi-region deployment
- Reference classes can be region-specific
- Low latency (<200ms) for users in US West Coast (MVP)
- **Rationale:** Construction domain is region-specific, architecture must support it

## Reliability & Availability

**NFR-R1: Uptime**
- Target 99.5% uptime (43.8 hours downtime/year allowed)
- Graceful degradation if LLM unavailable (return estimates without narrative)
- Graceful degradation if MCP functions unavailable (fallback to direct MongoDB)
- **Rationale:** MVP SLA, not mission-critical yet

**NFR-R2: Data Durability**
- MongoDB Atlas automatic backups (point-in-time recovery)
- Backup retention: 7 days
- Disaster recovery plan documented
- **Rationale:** Customer estimation data must not be lost

**NFR-R3: Error Recovery**
- Failed estimate requests return helpful error messages
- Partial failures (e.g., LLM timeout) return best-effort estimate
- Retry logic for transient failures (MongoDB connection, LLM API)
- **Rationale:** User experience should not break on transient errors

## Monitoring & Observability

**NFR-M1: Logging**
- Structured JSON logs with request_id, tenant_id, timestamp
- Log levels: DEBUG, INFO, WARNING, ERROR
- Retain logs for 30 days (compliance requirement)
- **Tools:** DigitalOcean Functions logs, MongoDB Atlas logs

**NFR-M2: Metrics & Alerting**
- Track: request rate, error rate, response time (p50/p95/p99)
- Alert on: error rate >5%, response time p95 >1s, rate limit hits
- Dashboard shows: tenant activity, reference class usage, feedback rate
- **Tools:** DigitalOcean monitoring, custom metrics API

**NFR-M3: Audit Logging**
- Log all tenant configuration changes
- Log all reference class tuning approvals
- Log all BYOK key changes
- Audit logs retained for 1 year
- **Compliance:** Required for SOC 2, GDPR

## Maintainability

**NFR-MA1: Code Quality**
- Python: Black formatting, Flake8 linting, type hints (mypy)
- JavaScript: ESLint, Prettier formatting
- Test coverage >70% for critical paths (RCF engine, auth)
- **Rationale:** Brownfield project needs standards for additions

**NFR-MA2: Documentation**
- OpenAPI (Swagger) docs auto-generated from FastAPI
- API endpoint documentation includes examples
- Data model documentation auto-generated from Pydantic
- Widget integration guide with working examples
- **Rationale:** External tenants need clear integration docs

**NFR-MA3: Deployment**
- Zero-downtime deployments (rolling updates)
- Database migrations applied automatically (pre-deployment)
- Rollback plan for failed deployments
- Staging environment mirrors production
- **Rationale:** Minimize deployment risk

## Usability (Widget Specific)

**NFR-U1: Widget Loading Performance**
- Widget JavaScript bundle <50KB gzipped
- Initial render <1 second on 3G connection
- Lazy load assets (images, fonts)
- **Rationale:** Customer-facing widget on contractor sites

**NFR-U2: Mobile Responsiveness**
- Widget works on mobile devices (iOS/Android)
- Touch-friendly buttons (min 44x44px)
- Keyboard input optimized for mobile
- **Rationale:** 60%+ of contractor website traffic is mobile

**NFR-U3: Accessibility (Basic)**
- WCAG 2.1 Level A compliance (widget)
- Keyboard navigation supported
- Screen reader compatible (ARIA labels)
- **Rationale:** Basic accessibility, not mission-critical for MVP

## Integration

**NFR-I1: LLM Provider Integration**
- OpenAI API: GPT-4 via REST API
- Support structured outputs (JSON mode)
- Handle rate limiting gracefully (429 responses)
- **Future:** Architecture supports swapping LLM providers

**NFR-I2: Email Integration**
- Transactional email for: signup, feedback links, notifications
- Email service: SendGrid or similar
- Template management for branded emails
- **Rationale:** Critical for feedback loop

**NFR-I3: Analytics Integration (Future)**
- Architecture supports event tracking (Mixpanel, Segment)
- Track: widget interactions, estimate conversions, feedback submissions
- **MVP:** Basic logging only, dedicated analytics post-MVP

---
