# Story 3.6: Create MongoDB Indexes for Tenant Isolation

Status: backlog

## Story

As a system,
I want MongoDB indexes optimized for tenant-scoped queries,
So that multi-tenant queries are fast even with millions of documents.

## Acceptance Criteria

**Given** all queries include tenant_id as first filter
**When** I create database indexes
**Then** migration script `app/db/migrations/001_tenant_indexes.py` creates indexes for:
- Tenants collection (email, api_key_hash)
- Reference classes (tenant_id + category + regions, tenant_id + keywords)
- Estimates (tenant_id + created_at, tenant_id + estimate_id)
- Feedback (tenant_id + estimate_id, tenant_id + created_at)

**And** index creation is idempotent (safe to run multiple times)

**And** query explain plans show index usage for tenant-scoped queries

**And** query performance: tenant-scoped queries < 50ms (p95)

## Tasks / Subtasks

- [ ] Create migration script for tenant indexes
- [ ] Add indexes for tenants collection
- [ ] Add indexes for reference_classes collection
- [ ] Add indexes for estimates collection
- [ ] Add indexes for feedback collection
- [ ] Make index creation idempotent
- [ ] Test query performance with indexes
- [ ] Verify explain plans use indexes

## Dev Notes

### Prerequisites

Stories 2.1, 3.1 (collections exist)

### Technical Notes

- Run migrations on app startup (check if indexes exist first)
- Compound indexes with tenant_id first enable efficient tenant isolation
- Monitor index size and query patterns in MongoDB Atlas
- For null tenant_id (platform data), use sparse indexes if needed

### References

- [Source: docs/epics.md#Story-3-6]
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
