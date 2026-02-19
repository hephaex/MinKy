#!/usr/bin/env bash
# MinKy 환경 검증 스크립트
# 사용법: ./scripts/check-env.sh [--full]

set -euo pipefail

# 색상 코드
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASS=0
WARN=0
FAIL=0
FULL_CHECK="${1:-}"

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  MinKy Environment Check${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
}

pass() {
    echo -e "  ${GREEN}[PASS]${NC} $1"
    PASS=$((PASS + 1))
}

warn() {
    echo -e "  ${YELLOW}[WARN]${NC} $1"
    WARN=$((WARN + 1))
}

fail() {
    echo -e "  ${RED}[FAIL]${NC} $1"
    FAIL=$((FAIL + 1))
}

section() {
    echo ""
    echo -e "${BLUE}--- $1 ---${NC}"
}

# ============================================================
# 1. 필수 도구 확인
# ============================================================
section "Required Tools"

if command -v rustc &>/dev/null; then
    RUST_VERSION=$(rustc --version 2>&1 | awk '{print $2}')
    pass "Rust: $RUST_VERSION"
else
    fail "Rust not installed (required for backend)"
fi

if command -v cargo &>/dev/null; then
    pass "Cargo: $(cargo --version 2>&1 | awk '{print $2}')"
else
    fail "Cargo not installed"
fi

if command -v node &>/dev/null; then
    NODE_VERSION=$(node --version 2>&1)
    pass "Node.js: $NODE_VERSION"
else
    warn "Node.js not installed (required for frontend)"
fi

if command -v psql &>/dev/null; then
    PG_VERSION=$(psql --version 2>&1 | awk '{print $3}')
    pass "PostgreSQL client: $PG_VERSION"
else
    warn "PostgreSQL client not found (psql)"
fi

if command -v sqlx &>/dev/null; then
    pass "sqlx-cli: installed"
else
    warn "sqlx-cli not installed (needed for migrations)"
    echo "         Install: cargo install sqlx-cli --features postgres"
fi

# ============================================================
# 2. .env 파일 확인
# ============================================================
section "Environment Variables"

ENV_FILE="minky-rust/.env"

if [ -f "$ENV_FILE" ]; then
    pass ".env file found: $ENV_FILE"

    # DATABASE_URL
    if grep -q "DATABASE_URL" "$ENV_FILE" && [ -n "$(grep 'DATABASE_URL' "$ENV_FILE" | cut -d= -f2-)" ]; then
        DB_URL=$(grep "DATABASE_URL" "$ENV_FILE" | cut -d= -f2-)
        pass "DATABASE_URL: set (${DB_URL%%@*}@...)"
    else
        fail "DATABASE_URL: not set in .env"
    fi

    # JWT_SECRET
    if grep -q "JWT_SECRET" "$ENV_FILE" && [ -n "$(grep 'JWT_SECRET' "$ENV_FILE" | cut -d= -f2-)" ]; then
        JWT_VAL=$(grep "JWT_SECRET" "$ENV_FILE" | cut -d= -f2-)
        if [ ${#JWT_VAL} -ge 32 ]; then
            pass "JWT_SECRET: set (length: ${#JWT_VAL} chars)"
        else
            warn "JWT_SECRET: too short (${#JWT_VAL} chars, recommend >= 32)"
        fi
    else
        fail "JWT_SECRET: not set in .env"
    fi

    # OPENAI_API_KEY
    if grep -q "OPENAI_API_KEY" "$ENV_FILE" && [ -n "$(grep 'OPENAI_API_KEY' "$ENV_FILE" | grep -v '^#' | cut -d= -f2-)" ]; then
        OPENAI_KEY=$(grep "OPENAI_API_KEY" "$ENV_FILE" | grep -v '^#' | cut -d= -f2-)
        if [[ "$OPENAI_KEY" == sk-* ]]; then
            pass "OPENAI_API_KEY: set (sk-...${OPENAI_KEY: -4})"
        else
            warn "OPENAI_API_KEY: set but format looks unexpected"
        fi
    else
        warn "OPENAI_API_KEY: not set (needed for embeddings)"
    fi

    # ANTHROPIC_API_KEY
    if grep -q "ANTHROPIC_API_KEY" "$ENV_FILE" && [ -n "$(grep 'ANTHROPIC_API_KEY' "$ENV_FILE" | grep -v '^#' | cut -d= -f2-)" ]; then
        ANTH_KEY=$(grep "ANTHROPIC_API_KEY" "$ENV_FILE" | grep -v '^#' | cut -d= -f2-)
        if [[ "$ANTH_KEY" == sk-ant-* ]]; then
            pass "ANTHROPIC_API_KEY: set (sk-ant-...${ANTH_KEY: -4})"
        else
            warn "ANTHROPIC_API_KEY: set but format looks unexpected"
        fi
    else
        warn "ANTHROPIC_API_KEY: not set (needed for document understanding)"
    fi

    # SERVER_PORT
    if grep -q "SERVER_PORT" "$ENV_FILE"; then
        PORT=$(grep "SERVER_PORT" "$ENV_FILE" | cut -d= -f2-)
        pass "SERVER_PORT: $PORT"
    else
        warn "SERVER_PORT: not set (default: 8000)"
    fi
else
    fail ".env file not found at $ENV_FILE"
    echo "         Copy template: cp minky-rust/.env.example minky-rust/.env"
fi

# ============================================================
# 3. 데이터베이스 연결 확인
# ============================================================
section "Database Connection"

if [ -f "$ENV_FILE" ] && grep -q "DATABASE_URL" "$ENV_FILE"; then
    DB_URL=$(grep "DATABASE_URL" "$ENV_FILE" | grep -v '^#' | cut -d= -f2-)

    if [ -n "$DB_URL" ] && command -v psql &>/dev/null; then
        if psql "$DB_URL" -c "SELECT 1;" &>/dev/null 2>&1; then
            pass "Database connection: OK"

            # pgvector 확장 확인
            if psql "$DB_URL" -c "SELECT extname FROM pg_extension WHERE extname = 'vector';" 2>/dev/null | grep -q "vector"; then
                pass "pgvector extension: installed"
            else
                warn "pgvector extension: NOT installed"
                echo "         Install: CREATE EXTENSION IF NOT EXISTS vector;"
            fi

            # 마이그레이션 확인
            if psql "$DB_URL" -c "SELECT COUNT(*) FROM _sqlx_migrations;" &>/dev/null 2>&1; then
                MIGRATION_COUNT=$(psql "$DB_URL" -t -c "SELECT COUNT(*) FROM _sqlx_migrations;" 2>/dev/null | tr -d ' ')
                pass "Migrations applied: $MIGRATION_COUNT"
            else
                warn "Migration table not found (run: sqlx migrate run)"
            fi
        else
            fail "Database connection: FAILED"
            echo "         Check DATABASE_URL in $ENV_FILE"
        fi
    else
        warn "Database connection: skipped (psql not available or DATABASE_URL empty)"
    fi
else
    warn "Database connection: skipped (DATABASE_URL not configured)"
fi

# ============================================================
# 4. Rust 빌드 확인
# ============================================================
section "Rust Build"

if [ -f "minky-rust/Cargo.toml" ]; then
    echo "  Checking Rust build (this may take a moment)..."
    if cargo build --manifest-path minky-rust/Cargo.toml 2>/dev/null; then
        pass "Rust build: OK"
    else
        fail "Rust build: FAILED"
        echo "         Run: cargo build --manifest-path minky-rust/Cargo.toml"
    fi

    if [ "$FULL_CHECK" = "--full" ]; then
        echo "  Running Rust tests..."
        TEST_OUTPUT=$(cargo test --manifest-path minky-rust/Cargo.toml 2>&1 | tail -5)
        if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
            TEST_COUNT=$(echo "$TEST_OUTPUT" | grep "test result" | grep -oP '\d+ passed' | head -1)
            pass "Rust tests: $TEST_COUNT"
        else
            fail "Rust tests: FAILED"
        fi
    fi
else
    warn "Rust backend: minky-rust/Cargo.toml not found"
fi

# ============================================================
# 5. 프론트엔드 확인
# ============================================================
section "Frontend"

if [ -f "frontend/package.json" ]; then
    pass "package.json: found"

    if [ -d "frontend/node_modules" ]; then
        pass "node_modules: installed"
    else
        warn "node_modules: not installed (run: cd frontend && npm install)"
    fi

    if [ "$FULL_CHECK" = "--full" ] && command -v node &>/dev/null; then
        echo "  Running frontend tests..."
        if cd frontend && CI=true npm test -- --watchAll=false --passWithNoTests 2>/dev/null | grep -q "Tests:"; then
            pass "Frontend tests: OK"
        else
            warn "Frontend tests: could not determine status"
        fi
        cd ..
    fi
else
    warn "Frontend: frontend/package.json not found"
fi

# ============================================================
# 6. 서버 상태 확인 (실행 중인 경우)
# ============================================================
section "Running Services"

BACKEND_PORT=8000
FRONTEND_PORT=3000

if command -v curl &>/dev/null; then
    if curl -s "http://localhost:$BACKEND_PORT/api/health" 2>/dev/null | grep -q '"status"'; then
        HEALTH_STATUS=$(curl -s "http://localhost:$BACKEND_PORT/api/health" 2>/dev/null)
        pass "Rust backend (port $BACKEND_PORT): running"
        if echo "$HEALTH_STATUS" | grep -q '"database":"healthy"'; then
            pass "  Database health: healthy"
        else
            warn "  Database health: degraded"
        fi
    else
        warn "Rust backend: not running on port $BACKEND_PORT"
        echo "         Start: cargo run --manifest-path minky-rust/Cargo.toml"
    fi

    if curl -s "http://localhost:$FRONTEND_PORT" &>/dev/null; then
        pass "Frontend (port $FRONTEND_PORT): running"
    else
        warn "Frontend: not running on port $FRONTEND_PORT"
        echo "         Start: cd frontend && npm start"
    fi
else
    warn "curl not available, skipping service health check"
fi

# ============================================================
# 요약
# ============================================================
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "  ${GREEN}PASS${NC}: $PASS"
echo -e "  ${YELLOW}WARN${NC}: $WARN"
echo -e "  ${RED}FAIL${NC}: $FAIL"
echo ""

if [ $FAIL -gt 0 ]; then
    echo -e "${RED}Environment has $FAIL critical issues. Fix them before starting.${NC}"
    exit 1
elif [ $WARN -gt 0 ]; then
    echo -e "${YELLOW}Environment is functional but has $WARN warnings.${NC}"
    echo "  Some features (embeddings, document understanding) may not work."
    exit 0
else
    echo -e "${GREEN}Environment is fully configured and ready.${NC}"
    exit 0
fi
