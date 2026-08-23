# Command source of truth. CI and contributors invoke `just <recipe>`.

# Default recipe: show available recipes.
default:
    @just --list

fmt:
    pnpm --filter n2a2ui fmt

fmt-check:
    pnpm --filter n2a2ui fmt:check

lint:
    pnpm --filter n2a2ui lint

typecheck:
    pnpm --filter n2a2ui typecheck

build:
    pnpm --filter n2a2ui build

test:
    pnpm --filter n2a2ui test

ci: fmt-check lint typecheck test build

# Hits the real Notion API. Requires NOTION_API_KEY and BLOCK_ID.
test-live:
    pnpm --filter n2a2ui test:live

ci-live: ci test-live

coverage:
    pnpm --filter n2a2ui coverage

coverage-ci:
    pnpm --filter n2a2ui coverage --coverage.reporter=lcov
    cp packages/n2a2ui/coverage/lcov.info lcov.info
