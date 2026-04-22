# Story 3.2: Implement JWT Authentication & Token Management

Status: backlog

## Story

As a developer,
I want JWT-based authentication for all API endpoints,
So that requests are authenticated and tenant_id is available in all operations.

## Acceptance Criteria

**Given** the architecture specifies JWT with tenant_id claim
**When** I implement authentication
**Then** `app/core/security.py` contains:
- `create_access_token(user_id, tenant_id, role)` function
- `verify_token(token)` function with expiration check
- JWT secret key from environment variable
- Token expiration: 24 hours (access), 30 days (refresh)

**And** JWT token structure includes:
```python
{
  "sub": "user_id_123",
  "tenant_id": "tenant_abc",
  "role": "admin",
  "exp": 1699564800
}
```

**And** `app/api/v1/dependencies.py` contains:
- `get_current_user()` dependency that extracts user from JWT
- `get_tenant_id()` dependency that extracts tenant_id from JWT
- Raises 401 Unauthorized if token invalid/expired

**And** `POST /api/v1/auth/login` endpoint:
- Accepts email + password
- Verifies credentials
- Returns access_token and refresh_token

**And** all protected endpoints use `tenant_id = Depends(get_tenant_id)` pattern

## Tasks / Subtasks

- [ ] Create `app/core/security.py` with JWT functions
- [ ] Implement `create_access_token()` function
- [ ] Implement `verify_token()` function
- [ ] Create `app/api/v1/dependencies.py`
- [ ] Implement `get_current_user()` dependency
- [ ] Implement `get_tenant_id()` dependency
- [ ] Create `POST /api/v1/auth/login` endpoint
- [ ] Add rate limiting to login endpoint
- [ ] Test authentication flow
- [ ] Test token expiration handling

## Dev Notes

### Prerequisites

Story 3.1 (tenant model exists)

### Technical Notes

- Use python-jose[cryptography] for JWT
- Include rate limit check in login endpoint (5 attempts per 15 min)
- Log all authentication attempts (success and failure)
- Consider refresh token rotation for security

### References

- [Source: docs/epics.md#Story-3-2]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

<!-- To be filled by dev agent -->

### Debug Log References

<!-- To be filled by dev agent during implementation -->

### Completion Notes List

<!-- To be filled by dev agent upon completion -->

### File List

<!-- NEW/MODIFIED/DELETED files will be listed here by dev agent -->
