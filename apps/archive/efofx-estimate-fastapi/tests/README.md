# Tests Directory

This directory contains all unit and integration tests for the efOfX Estimation Service.

## Structure

- **conftest.py**: Pytest configuration and shared fixtures
- **fixtures/**: Test data and fixtures
- **api/**: API endpoint tests
- **services/**: Service layer tests
- **unit/**: Unit tests for individual components
- **integration/**: Integration tests

## Test Categories

### Unit Tests
- Individual function testing
- Service method testing
- Utility function testing
- Model validation testing

### Integration Tests
- API endpoint testing
- Database integration testing
- LLM integration testing
- End-to-end workflow testing

### Fixtures
- Sample data for testing
- Database setup/teardown
- Mock services and clients
- Test configuration

## Running Tests

```bash
# Run all tests
pytest

# Run only unit tests
pytest tests/unit/

# Run only integration tests
pytest tests/integration/

# Run with coverage
pytest --cov=app

# Run specific test file
pytest tests/api/test_estimation.py
```

## Test Configuration

Tests use a separate test database to avoid affecting production data. The test database is automatically created and cleaned up between test runs.

## Mocking

External dependencies (like OpenAI API) are mocked in tests to ensure reliable and fast test execution. Mock responses are defined in the fixtures directory. 