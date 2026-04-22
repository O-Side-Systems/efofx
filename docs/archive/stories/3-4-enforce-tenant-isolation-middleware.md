# Story 3.4: Enforce Tenant Isolation Middleware

Status: backlog

## Story

As a system,
I want ALL database queries to include tenant_id filter automatically,
So that cross-tenant data leakage is impossible.

## Acceptance Criteria

**Given** the architecture mandates 100% tenant_id scoping
**When** I create tenant isolation middleware
**Then** `app/middleware/tenant_isolation.py` contains:
- Middleware that extracts tenant_id from JWT on every request
- Attaches tenant_id to request state: `request.state.tenant_id`
- Raises 401 if tenant_id missing (except for public endpoints like login)

**And** all service methods accept tenant_id as FIRST parameter:
```python
async def get_reference_classes(tenant_id: str, category: str):
    return await db.reference_classes.find({
        "tenant_id": tenant_id,  # REQUIRED
        "category": category
    }).to_list(None)
```

**And** MongoDB queries ALWAYS include tenant_id:
```python
# ✅ CORRECT
{"tenant_id": tenant_id, "status": "active"}

# ❌ WRONG - will fail code review
{"status": "active"}
```

**And** code review checklist includes: "Does this query filter by tenant_id?"

**And** integration tests verify tenant A cannot access tenant B's data

## Tasks / Subtasks

- [ ] Create `app/middleware/tenant_isolation.py`
- [ ] Implement middleware to extract tenant_id from JWT
- [ ] Attach tenant_id to request state
- [ ] Add public endpoint whitelist (login, health check)
- [ ] Update all service methods to accept tenant_id as first parameter
- [ ] Update all database queries to include tenant_id filter
- [ ] Create helper function `get_tenant_filter()`
- [ ] Write integration tests for tenant isolation
- [ ] Document code review checklist

## Dev Notes

### Prerequisites

Story 3.2 (JWT includes tenant_id)

### Technical Notes

- Follow architecture: Cross-Cutting Concerns → Multi-Tenant Isolation
- Public endpoints (login, health check) bypass tenant_id requirement
- Create helper function: `get_tenant_filter(tenant_id, **kwargs)` that merges filters
- Write pytest fixtures for tenant isolation testing

### References

- [Source: docs/epics.md#Story-3-4]
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
