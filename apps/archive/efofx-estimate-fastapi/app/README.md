# App Directory

This directory contains the main application code for the efOfX Estimation Service.

## Structure

- **api/**: FastAPI route handlers and API endpoints
- **core/**: Configuration, security, constants, and core utilities
- **db/**: Database connection and data access layer
- **models/**: Pydantic data models and schemas
- **services/**: Business logic services
- **utils/**: Helper functions and utilities
- **main.py**: FastAPI application entry point

## Key Components

### API Layer (`api/`)
- Route handlers for all HTTP endpoints
- Request/response validation
- Authentication and authorization
- Error handling

### Core (`core/`)
- Application configuration management
- Security utilities and authentication
- Constants and enums
- Environment variable handling

### Database (`db/`)
- MongoDB connection management
- Collection access utilities
- Database health checks
- Index creation

### Models (`models/`)
- Pydantic schemas for data validation
- Request/response models
- Database document models
- Type definitions

### Services (`services/`)
- Business logic implementation
- Estimation processing
- LLM integration
- Chat functionality
- Feedback management

### Utils (`utils/`)
- File handling utilities
- Validation functions
- Calculation helpers
- Common utilities

## Development

All business logic should be implemented in the services layer, with API routes acting as thin controllers that delegate to services. Models provide data validation and serialization throughout the application. 