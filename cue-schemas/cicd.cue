// Moon CICD Pipeline Specification
// Defines all CI/CD tasks, pipelines, and quality gates
// Source: moon.yml (355 lines)
package isolate

// ═══════════════════════════════════════════════════════════════════════════
// MOON CICD SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

#MoonTask: {
    name:         string & !=""
    description:  string & !=""
    command:      string & !=""
    cache:        bool | *true
    inputs:       [...string] | *[]
    outputs:      [...string] | *[]
    dependsOn:    [...string] | *[]
    stage:        #Stage
    duration_s:   int & >=1 & <=300 | *10
}

#CompositeTask: {
    name:        string & !=""
    description: string & !=""
    stage:       "orchestration"
    dependsOn:   [...string]
    tasks:       int  // count of dependent tasks
    duration_s:  int & >=1 & <=300
}

#Stage: "formatting" | "linting" | "testing" | "mutation" | "llm-review" | "security" | "build" | "deployment" | "orchestration"

// ═══════════════════════════════════════════════════════════════════════════
// STAGE 1: CODE FORMATTING & LINTING (Fast ~10-15s)
// ═══════════════════════════════════════════════════════════════════════════

moon_tasks: {
    // Stage 1a: Formatting
    fmt: #MoonTask & {
        name:        "fmt"
        description: "Check code formatting (rustfmt)"
        command:     "cargo fmt --all --check"
        cache:       false
        inputs:      ["src/**/*.rs", "Cargo.toml", "rustfmt.toml"]
        outputs:     []
        stage:       "formatting"
        duration_s:  2
    }

    fmt_fix: #MoonTask & {
        name:        "fmt-fix"
        description: "Auto-fix code formatting"
        command:     "cargo fmt --all"
        cache:       false
        inputs:      ["src/**/*.rs", "Cargo.toml"]
        outputs:     []
        stage:       "formatting"
        duration_s:  2
    }

    // Stage 1b: Linting
    clippy: #MoonTask & {
        name:        "clippy"
        description: "Lint with Clippy (strict mode: -D warnings)"
        command:     "cargo clippy --workspace --all-targets --all-features -- -D warnings"
        cache:       true
        inputs:      ["src/**/*.rs", "Cargo.toml", ".clippy.toml"]
        outputs:     ["target/"]
        dependsOn:   ["fmt"]
        stage:       "linting"
        duration_s:  8
    }

    lint: #MoonTask & {
        name:        "lint"
        description: "Check documentation completeness"
        command:     "cargo doc --no-deps --document-private-items 2>&1 | grep -E '(warning|error)' || true"
        cache:       true
        inputs:      ["src/**/*.rs", "Cargo.toml"]
        outputs:     []
        stage:       "linting"
        duration_s:  6
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STAGE 2: UNIT & PROPERTY-BASED TESTS (30-45s)
    // ═══════════════════════════════════════════════════════════════════════════

    test: #MoonTask & {
        name:        "test"
        description: "Run all unit tests"
        command:     "cargo test --workspace --all-features"
        cache:       true
        inputs:      ["src/**/*.rs", "Cargo.toml", "tests/**/*"]
        outputs:     ["target/test-results.json"]
        dependsOn:   ["fmt", "clippy"]
        stage:       "testing"
        duration_s:  25
    }

    test_doc: #MoonTask & {
        name:        "test-doc"
        description: "Run documentation tests"
        command:     "cargo test --doc --workspace --all-features"
        cache:       true
        inputs:      ["src/**/*.rs", "Cargo.toml"]
        outputs:     []
        dependsOn:   ["fmt"]
        stage:       "testing"
        duration_s:  4
    }

    test_properties: #MoonTask & {
        name:        "test-properties"
        description: "Run property-based tests (proptest 10,000 cases)"
        command:     "cargo test --test '*' --features proptest --workspace --all-features -- --test-threads 1"
        cache:       true
        inputs:      ["src/**/*.rs", "tests/**/*.rs", "Cargo.toml"]
        outputs:     ["target/proptest-results.json"]
        dependsOn:   ["test"]
        stage:       "testing"
        duration_s:  40
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STAGE 3: MUTATION TESTING (2-5 minutes)
    // ═══════════════════════════════════════════════════════════════════════════

    mutants: #MoonTask & {
        name:        "mutants"
        description: "Verify test quality via mutation testing"
        command:     "sh .moon/scripts/mutation-test.sh"
        cache:       true
        inputs:      ["src/**/*.rs", "tests/**/*.rs", "Cargo.toml"]
        outputs:     ["target/mutants.json", ".mutations-report/"]
        dependsOn:   ["test"]
        stage:       "mutation"
        duration_s:  180  // 3 minutes typical
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STAGE 4: LLM-AS-JUDGE CODE REVIEW (30-60s)
    // ═══════════════════════════════════════════════════════════════════════════

    llm_judge: #MoonTask & {
        name:        "llm-judge"
        description: "LLM code review (Claude as judge)"
        command:     "python3 .moon/scripts/llm-judge.py"
        cache:       false  // Always runs
        inputs:      ["src/**/*.rs", "Cargo.toml"]
        outputs:     [".llm-review-report.json", ".llm-review-report.md"]
        dependsOn:   ["test"]
        stage:       "llm-review"
        duration_s:  45
    }

    llm_judge_fix_suggestions: #MoonTask & {
        name:        "llm-judge-fix-suggestions"
        description: "Generate LLM-based improvement suggestions"
        command:     "python3 .moon/scripts/llm-judge.py --suggest-fixes"
        cache:       false  // Always runs
        inputs:      ["src/**/*.rs", "Cargo.toml"]
        outputs:     [".llm-suggestions.json", ".llm-suggestions.md"]
        dependsOn:   []
        stage:       "llm-review"
        duration_s:  45
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STAGE 5: SECURITY & DEPENDENCY CHECKS (15-30s)
    // ═══════════════════════════════════════════════════════════════════════════

    audit: #MoonTask & {
        name:        "audit"
        description: "Security audit of dependencies"
        command:     "cargo audit --deny warnings"
        cache:       true
        inputs:      ["Cargo.lock", "Cargo.toml"]
        outputs:     []
        stage:       "security"
        duration_s:  10
    }

    deps_check: #MoonTask & {
        name:        "deps-check"
        description: "Check for duplicate dependencies"
        command:     "cargo tree --duplicates"
        cache:       true
        inputs:      ["Cargo.lock", "Cargo.toml"]
        outputs:     []
        stage:       "security"
        duration_s:  3
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STAGE 6: BUILD & ARTIFACTS (45-90s)
    // ═══════════════════════════════════════════════════════════════════════════

    build: #MoonTask & {
        name:        "build"
        description: "Build release binaries"
        command:     "cargo build --release --workspace --all-features"
        cache:       true
        inputs:      ["src/**/*.rs", "Cargo.toml", "Cargo.lock"]
        outputs:     ["target/release", "bin/"]
        dependsOn:   ["clippy", "test"]
        stage:       "build"
        duration_s:  75  // First run 90s, cached 30-45s
    }

    build_docs: #MoonTask & {
        name:        "build-docs"
        description: "Generate Rust documentation"
        command:     "cargo doc --release --no-deps --document-private-items --all-features"
        cache:       true
        inputs:      ["src/**/*.rs", "Cargo.toml"]
        outputs:     ["target/doc"]
        dependsOn:   ["clippy"]
        stage:       "build"
        duration_s:  25
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STAGE 7: CONTINUOUS DEPLOYMENT GATES
    // ═══════════════════════════════════════════════════════════════════════════

    cd_gates: #MoonTask & {
        name:        "cd-gates"
        description: "Verify CD readiness (deployment prerequisites)"
        command:     "sh .moon/scripts/cd-gates.sh"
        cache:       false  // Always runs
        inputs:      ["src/**/*.rs", "Cargo.toml"]
        outputs:     [".cd-gates-report.json", ".cd-gates-report.md"]
        dependsOn:   ["test", "build", "llm-judge"]
        stage:       "deployment"
        duration_s:  10
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONVENIENCE COMMANDS
    // ═══════════════════════════════════════════════════════════════════════════

    clean: #MoonTask & {
        name:        "clean"
        description: "Clean build artifacts and reports"
        command:     "cargo clean && rm -rf .llm-*.json .llm-*.md .cd-gates-* .mutations-report"
        cache:       false
        inputs:      []
        outputs:     []
        stage:       "orchestration"
        duration_s:  5
    }

    logs: #MoonTask & {
        name:        "logs"
        description: "Display quality reports from last run"
        command:     "sh .moon/scripts/show-reports.sh"
        cache:       false
        inputs:      []
        outputs:     []
        stage:       "orchestration"
        duration_s:  2
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPOSITE PIPELINES (ORCHESTRATION)
// ═══════════════════════════════════════════════════════════════════════════

moon_pipelines: {
    quick: #CompositeTask & {
        name:        "quick"
        description: "Fast lint check (format + clippy)"
        dependsOn:   ["fmt", "clippy"]
        tasks:       2
        duration_s:  15
    }

    quality: #CompositeTask & {
        name:        "quality"
        description: "All quality gates (no build)"
        dependsOn:   ["fmt", "clippy", "lint", "test", "test-doc", "audit", "deps-check", "llm-judge"]
        tasks:       8
        duration_s:  60
    }

    ci: #CompositeTask & {
        name:        "ci"
        description: "Complete CI pipeline (lint, test, build, quality)"
        dependsOn:   ["quick", "test", "test-properties", "mutants", "build", "build-docs", "audit", "llm-judge", "cd-gates"]
        tasks:       9
        duration_s:  180  // 3 minutes
    }

    deploy: #CompositeTask & {
        name:        "deploy"
        description: "Full CI pipeline + deployment readiness checks"
        dependsOn:   ["ci", "cd-gates"]
        tasks:       10
        duration_s:  180
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STAGE DEFINITIONS & QUALITY GATES
// ═══════════════════════════════════════════════════════════════════════════

stages: {
    stage_1_formatting: {
        name:        "Code Formatting & Linting"
        duration:    "~10-15s"
        tasks:       ["fmt", "fmt-fix", "clippy", "lint"]
        auto_fixable: true
        purpose:     "Enforce code style and basic linting"
    }

    stage_2_testing: {
        name:        "Unit & Property-Based Testing"
        duration:    "~30-45s"
        tasks:       ["test", "test-doc", "test-properties"]
        auto_fixable: false
        purpose:     "Verify code correctness with comprehensive coverage"
    }

    stage_3_mutation: {
        name:        "Mutation Testing"
        duration:    "~2-5 min"
        tasks:       ["mutants"]
        auto_fixable: false
        purpose:     "Verify test suite quality by introducing mutations"
    }

    stage_4_llm_review: {
        name:        "LLM-as-Judge Code Review"
        duration:    "~30-60s"
        tasks:       ["llm-judge", "llm-judge-fix-suggestions"]
        auto_fixable: false
        purpose:     "Architectural and design pattern validation"
    }

    stage_5_security: {
        name:        "Security & Dependency Checks"
        duration:    "~15-30s"
        tasks:       ["audit", "deps-check"]
        auto_fixable: false
        purpose:     "Identify vulnerabilities and dependency bloat"
    }

    stage_6_build: {
        name:        "Build & Artifacts"
        duration:    "~45-90s"
        tasks:       ["build", "build-docs"]
        auto_fixable: false
        purpose:     "Create production binaries and documentation"
    }

    stage_7_deployment: {
        name:        "Continuous Deployment Gates"
        duration:    "~5-15s"
        tasks:       ["cd-gates"]
        auto_fixable: false
        purpose:     "Final verification before deployment"
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MOON CONFIGURATION STRUCTURE
// ═══════════════════════════════════════════════════════════════════════════

#MoonConfig: {
    workspace_version: string | *"1.20"
    tasks:             int | *17  // individual tasks
    pipelines:         int | *4   // composite pipelines
    utilities:         int | *2   // convenience commands
    total_tasks:       int | *23
}

moon_config: #MoonConfig & {
    workspace_version: "1.20"
    tasks:             17
    pipelines:         4
    utilities:         2
    total_tasks:       23
}

// ═══════════════════════════════════════════════════════════════════════════
// LLM-AS-JUDGE EVALUATION CRITERIA
// ═══════════════════════════════════════════════════════════════════════════

#LLMEvaluation: {
    category: string & !=""
    criteria: [...string]
}

llm_judge_criteria: [...#LLMEvaluation] & [
    {
        category: "Design Patterns"
        criteria: [
            "Are architectural patterns idiomatic?",
            "Are there anti-patterns present?",
            "Does code follow domain-driven design?",
        ]
    },
    {
        category: "Error Handling"
        criteria: [
            "Are all error paths handled?",
            "Is error recovery comprehensive?",
            "Are error messages user-friendly?",
        ]
    },
    {
        category: "Functional Programming"
        criteria: [
            "Are FP idioms used correctly?",
            "Is immutability preserved?",
            "Are combinators used idiomatically?",
        ]
    },
    {
        category: "Type Safety"
        criteria: [
            "Are generics used properly?",
            "Is the type system leveraged fully?",
            "Are lifetimes necessary and correct?",
        ]
    },
    {
        category: "Performance"
        criteria: [
            "Are there obvious performance issues?",
            "Is memory usage appropriate?",
            "Are algorithms efficient?",
        ]
    },
    {
        category: "Security"
        criteria: [
            "Are there injection vulnerabilities?",
            "Is user input validated?",
            "Are cryptographic practices sound?",
        ]
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// MUTATION TESTING PHILOSOPHY
// ═══════════════════════════════════════════════════════════════════════════

mutation_testing: {
    purpose: "Verify that test suite can catch code mutations (introduced bugs)"

    philosophy: """
        If tests don't catch mutations, they're not comprehensive enough.
        Example: Change 'if x > 0' to 'if x >= 0' - tests must fail.
        """

    metrics: [
        "Mutation score (% of mutations killed)",
        "Coverage gaps (unmutated code)",
        "Test effectiveness per module",
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY-BASED TESTING
// ═══════════════════════════════════════════════════════════════════════════

property_based_testing: {
    framework: "proptest"
    cases_per_property: 10000
    purpose: "Find edge cases that example-based tests miss"

    example: """
        Instead of testing sort([1,2,3]) manually,
        generate 10,000 random arrays and verify sort correctness on each.
        """
}

// ═══════════════════════════════════════════════════════════════════════════
// PERFORMANCE METRICS
// ═══════════════════════════════════════════════════════════════════════════

performance_metrics: {
    quick:       {first_run: 15, cached: 5}     // seconds
    test:        {first_run: 45, cached: 25}
    build:       {first_run: 90, cached: 45}
    ci:          {first_run: 180, cached: 120}  // 3 min / 2 min
    bottleneck:  "Mutation testing (2-5 min)"
}

// ═══════════════════════════════════════════════════════════════════════════
// USAGE RULES
// ═══════════════════════════════════════════════════════════════════════════

usage_rules: {
    golden_rule: "ALWAYS use 'moon run', NEVER use 'cargo' directly"

    correct_commands: [
        "moon run :ci",
        "moon run :test",
        "moon run :quick",
        "moon run :build",
        "moon run :quality",
        "moon run :deploy",
    ]

    incorrect_commands: [
        "❌ cargo fmt",
        "❌ cargo clippy",
        "❌ cargo test",
        "❌ cargo build",
    ]

    examples: {
        before_committing:  "moon run :quick"
        before_pushing:     "moon run :ci"
        code_review_ideas:  "moon run :llm-judge-fix-suggestions"
        ready_to_deploy:    "moon run :deploy"
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTINUOUS DEPLOYMENT (THOUGHTWORKS) PATTERN
// ═══════════════════════════════════════════════════════════════════════════

continuous_deployment: {
    workflow: [
        "Automated quality gates run on every change",
        "All gates must pass (7 stages)",
        "CD gates verify deployment prerequisites",
        "Manual approval (if configured)",
        "Automated deployment upon approval",
    ]

    stages: 7
    gates: [
        "Formatting",
        "Linting",
        "Testing (unit + property + docs)",
        "Mutation testing",
        "LLM code review",
        "Security & dependencies",
        "Build & artifacts",
        "Deployment readiness",
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// CACHING STRATEGY
// ═══════════════════════════════════════════════════════════════════════════

caching_strategy: {
    approach: "Input-based fingerprinting via moon"
    cache_location: "~/.moon/cache"

    cached_tasks: [
        "clippy", "lint",           // linting
        "test", "test-doc", "test-properties",  // testing
        "mutants",                  // mutation
        "audit", "deps-check",      // security
        "build", "build-docs",      // build
    ]

    never_cached: [
        "fmt",                      // always check
        "fmt-fix",                  // always apply
        "llm-judge",               // always evaluate
        "llm-judge-fix-suggestions", // always generate
        "cd-gates",                // always verify
        "clean",                   // utility
        "logs",                    // utility
    ]

    input_tracking: "src/**/*.rs, Cargo.toml, Cargo.lock, etc."
}

// ═══════════════════════════════════════════════════════════════════════════
// SUMMARY STATISTICS
// ═══════════════════════════════════════════════════════════════════════════

summary: {
    total_tasks:       23
    individual_tasks:  17
    composite_pipelines: 4
    utility_commands:  2

    tasks_by_stage: {
        formatting:  2
        linting:     2
        testing:     3
        mutation:    1
        llm_review:  2
        security:    2
        build:       2
        deployment:  1
        orchestration: 4
    }

    total_runtime: {
        ci_first_run: "2-3 minutes"
        ci_cached:    "1-2 minutes"
        bottleneck:   "Mutation testing"
    }
}
