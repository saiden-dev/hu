# Default recipe
default:
    @just --list

# Run all checks
check: format lint typecheck

# Format code with ruff
format:
    ruff format eks-shell.py

# Lint and autofix with ruff
lint:
    ruff check --fix eks-shell.py

# Run type checking with mypy
typecheck:
    mypy eks-shell.py
