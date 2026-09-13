#!/bin/bash
# validate-governance.sh — ASFDK Rust Governance Validation
# Run: bash .nltotoi/scripts/validate-governance.sh
# Returns: 0 if all checks pass, 1 if any fail

set -euo pipefail

OTOI_VERSION="ORG-DEV-OTOI-1.0.3"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PASS=0
FAIL=0

check() {
    local name="$1"
    shift
    if "$@" > /dev/null 2>&1; then
        echo "  ✅ PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  ❌ FAIL: $name"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== ASFDK Rust Governance Validation ==="
echo "Document ID: $OTOI_VERSION"
echo ""

# 1. OTOI contract
check "NLT-DEV-OTOI.md exists" test -f "$REPO_ROOT/NLT-DEV-OTOI.md"
check "OTOI version is 1.0.3" grep -q "ORG-DEV-OTOI-1.0.3" "$REPO_ROOT/NLT-DEV-OTOI.md"

# 2. Agent registry and protocols
check "AGENTS.md exists" test -f "$REPO_ROOT/AGENTS.md"
check "AGENTS.md has agent registry" grep -q "### Registered Agents" "$REPO_ROOT/AGENTS.md"
check "CLAUDE.md exists" test -f "$REPO_ROOT/CLAUDE.md"
check "CLAUDE.md references commit format" grep -q "type(scope): description" "$REPO_ROOT/CLAUDE.md"

# 3. Governance directory
check "nltotoi directory exists" test -d "$REPO_ROOT/.nltotoi"
check "nltotoi/README.md exists" test -f "$REPO_ROOT/.nltotoi/README.md"
check "nltotoi/index/governance-files.md exists" test -f "$REPO_ROOT/.nltotoi/index/governance-files.md"

# 4. Templates
check "templates/agent-registration.json exists" test -f "$REPO_ROOT/templates/agent-registration.json"
check "templates/handoff-record.json exists" test -f "$REPO_ROOT/templates/handoff-record.json"
check "templates/escalation.md exists" test -f "$REPO_ROOT/templates/escalation.md"
check "templates/intent-log.md exists" test -f "$REPO_ROOT/templates/intent-log.md"

# 5. Issue/PR templates and SOPs
check "ISSUE_TEMPLATE/agent-escalation.md exists" test -f "$REPO_ROOT/ISSUE_TEMPLATE/agent-escalation.md"
check "ISSUE_TEMPLATE/governance-proposal.md exists" test -f "$REPO_ROOT/ISSUE_TEMPLATE/governance-proposal.md"
check "PULL_REQUEST_TEMPLATE/agent-contribution.md exists" test -f "$REPO_ROOT/PULL_REQUEST_TEMPLATE/agent-contribution.md"
check "SOPs/new-agent-onboarding.md exists" test -f "$REPO_ROOT/SOPs/new-agent-onboarding.md"
check "SOPs/repo-governance-setup.md exists" test -f "$REPO_ROOT/SOPs/repo-governance-setup.md"
check "SOPs/incident-response.md exists" test -f "$REPO_ROOT/SOPs/incident-response.md"

# 6. CI and discovery manifest
check "workflows/validate-governance.yml exists" test -f "$REPO_ROOT/.github/workflows/validate-governance.yml"
check "nltotoi.json exists" test -f "$REPO_ROOT/nltotoi.json"
check "nltotoi.json declares the Rust repo" grep -q '"Rust"' "$REPO_ROOT/nltotoi.json"

# 7. Rust crate structure
check "Cargo.toml exists" test -f "$REPO_ROOT/Cargo.toml"
check "Cargo.toml declares the asfdk crate" grep -q 'name = "asfdk"' "$REPO_ROOT/Cargo.toml"
for f in types dto promptdefense sleepwalker rrt foundation tests; do
    check "src/$f.rs exists" test -f "$REPO_ROOT/src/$f.rs"
    check "lib.rs declares module $f" grep -q "mod $f;" "$REPO_ROOT/src/lib.rs"
done

# 8. Toolchain gates (run when cargo is available; skip with notice otherwise)
if command -v cargo > /dev/null 2>&1; then
    check "cargo fmt --check (formatting)" bash -c "cd \"$REPO_ROOT\" && cargo fmt --check"
    check "cargo build (compilation)" bash -c "cd \"$REPO_ROOT\" && cargo build --quiet"
    check "cargo clippy (lints)" bash -c "cd \"$REPO_ROOT\" && cargo clippy --all-targets --quiet -- -D warnings"
    check "cargo test (31 tests)" bash -c "cd \"$REPO_ROOT\" && cargo test --quiet"
else
    echo "  ⚠️  SKIP: cargo toolchain not on PATH — build/lint/test gates run in CI"
fi

echo ""
echo "=== Results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "❌ Governance validation FAILED — $FAIL check(s) failed"
    exit 1
else
    echo "✅ Governance validation PASSED — all $PASS checks OK"
    exit 0
fi
