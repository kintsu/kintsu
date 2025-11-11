-- USERS
insert into
    users (id, email, gh_id, gh_login, gh_avatar)
values
    (
        2,
        'user2@example.com',
        200001,
        'user2',
        'https://avatars.githubusercontent.com/u/200001?v=4'
    ),
    (
        3,
        'ld-corp-admin@example.com',
        200002,
        'ld-corp-admin',
        'https://avatars.githubusercontent.com/u/200002?v=4'
    ),
    (
        4,
        'metrics-maintainer@example.com',
        200003,
        'metrics-maintainer',
        'https://avatars.githubusercontent.com/u/200003?v=4'
    ),
    (
        5,
        'infra-admin@example.com',
        200004,
        'infra-admin',
        'https://avatars.githubusercontent.com/u/200004?v=4'
    );

select
    setval('users_id_seq', 6);

-- ORGANIZATIONS
insert into
    org (id, name, gh_id, gh_avatar)
values
    (
        1,
        'cadence-org',
        300001,
        'https://avatars.githubusercontent.com/u/300001?v=4'
    ),
    (
        2,
        'daisy-ch',
        300002,
        'https://avatars.githubusercontent.com/u/300002?v=4'
    ),
    (
        3,
        'ld-corp',
        300003,
        'https://avatars.githubusercontent.com/u/300003?v=4'
    ),
    (
        4,
        'metrics-labs',
        300004,
        'https://avatars.githubusercontent.com/u/300004?v=4'
    ),
    (
        5,
        'infra-tools',
        300005,
        'https://avatars.githubusercontent.com/u/300005?v=4'
    );

select
    setval('org_id_seq', 6);

-- ORG ADMINS
insert into
    org_admin (org_id, user_id, revoked_at)
values
    (1, 1, null),
    -- cadence-org: user 1 (developer)
    (2, 1, null),
    -- daisy-ch: user 1 (developer)
    (3, 3, null),
    -- ld-corp: user 3 (ld-corp-admin)
    (4, 4, null),
    -- metrics-labs: user 4 (metrics-maintainer)
    (5, 5, null);

-- infra-tools: user 5 (infra-admin)
-- PACKAGES
insert into
    package (id, name)
values
    (1, 'cadence-core'),
    -- ROOT
    (2, 'metrics-sdk'),
    -- ROOT
    (3, 'logger-base'),
    -- ROOT
    (4, 'cadence-apis'),
    -- INTERMEDIATE
    (5, 'logger-fmt'),
    -- INTERMEDIATE
    (6, 'metrics-aggregator'),
    -- INTERMEDIATE
    (7, 'daisy-vec'),
    -- LEAF
    (8, 'ld-geo'),
    -- LEAF (diamond)
    (9, 'observability-stack'),
    -- LEAF (linear end)
    (10, 'app-framework');

-- LEAF (multi-convergence)
select
    setval('package_id_seq', 11);

-- VERSIONS (25 total)
-- ROOT PACKAGE: cadence-core (3 versions)
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        1,
        1,
        '0.1.0',
        'sha256:cadence-core-0.1.0-source',
        'sha256:cadence-core-0.1.0-declarations',
        'Core utilities and data structures - Initial release',
        'https://cadence-org.dev/core',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# cadence-core\n\nCore utilities and data structures for the Cadence ecosystem.',
        'https://github.com/cadence-org/cadence-core',
        '{"utilities", "core", "data-structures"}',
        now() - interval '90 days',
        1,
        null,
        '{}'
    ),
    (
        2,
        1,
        '0.2.0',
        'sha256:cadence-core-0.2.0-source',
        'sha256:cadence-core-0.2.0-declarations',
        'Core utilities and data structures - Performance improvements',
        'https://cadence-org.dev/core',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# cadence-core\n\n## v0.2.0\n\nPerformance improvements and optimizations.',
        'https://github.com/cadence-org/cadence-core',
        '{"utilities", "core", "data-structures", "performance"}',
        now() - interval '60 days',
        1,
        null,
        '{}'
    ),
    (
        3,
        1,
        '0.3.0',
        'sha256:cadence-core-0.3.0-source',
        'sha256:cadence-core-0.3.0-declarations',
        'Core utilities and data structures - Added concurrent utilities',
        'https://cadence-org.dev/core',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# cadence-core\n\n## v0.3.0\n\nAdded concurrent utilities for multi-threaded applications.',
        'https://github.com/cadence-org/cadence-core',
        '{"utilities", "core", "data-structures", "concurrency"}',
        now() - interval '30 days',
        1,
        null,
        '{}'
    );

-- ROOT PACKAGE: metrics-sdk (3 versions)
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        4,
        2,
        '0.1.0',
        'sha256:metrics-sdk-0.1.0-source',
        'sha256:metrics-sdk-0.1.0-declarations',
        'Low-level metrics primitives - Basic counters and gauges',
        'https://metrics-labs.io/sdk',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# metrics-sdk\n\nLow-level metrics primitives for monitoring applications.',
        'https://github.com/metrics-labs/metrics-sdk',
        '{"metrics", "monitoring", "observability"}',
        now() - interval '85 days',
        4,
        null,
        '{}'
    ),
    (
        5,
        2,
        '0.2.0',
        'sha256:metrics-sdk-0.2.0-source',
        'sha256:metrics-sdk-0.2.0-declarations',
        'Low-level metrics primitives - Added histograms',
        'https://metrics-labs.io/sdk',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# metrics-sdk\n\n## v0.2.0\n\nAdded histogram support for distribution metrics.',
        'https://github.com/metrics-labs/metrics-sdk',
        '{"metrics", "monitoring", "observability", "histograms"}',
        now() - interval '55 days',
        4,
        null,
        '{}'
    ),
    (
        6,
        2,
        '0.2.1',
        'sha256:metrics-sdk-0.2.1-source',
        'sha256:metrics-sdk-0.2.1-declarations',
        'Low-level metrics primitives - Bugfix release',
        'https://metrics-labs.io/sdk',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# metrics-sdk\n\n## v0.2.1\n\nBugfix release: Fixed memory leak in histogram implementation.',
        'https://github.com/metrics-labs/metrics-sdk',
        '{"metrics", "monitoring", "observability", "histograms"}',
        now() - interval '25 days',
        4,
        null,
        '{}'
    );

-- ROOT PACKAGE: logger-base (3 versions)
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        7,
        3,
        '1.0.0',
        'sha256:logger-base-1.0.0-source',
        'sha256:logger-base-1.0.0-declarations',
        'Logging primitives and interfaces - Stable API',
        'https://infra-tools.org/logger',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# logger-base\n\nLogging primitives and interfaces for infrastructure tools.',
        'https://github.com/infra-tools/logger-base',
        '{"logging", "observability", "infrastructure"}',
        now() - interval '80 days',
        5,
        null,
        '{}'
    ),
    (
        8,
        3,
        '1.1.0',
        'sha256:logger-base-1.1.0-source',
        'sha256:logger-base-1.1.0-declarations',
        'Logging primitives and interfaces - Added structured logging',
        'https://infra-tools.org/logger',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# logger-base\n\n## v1.1.0\n\nAdded structured logging support with JSON output.',
        'https://github.com/infra-tools/logger-base',
        '{"logging", "observability", "infrastructure", "structured-logging"}',
        now() - interval '50 days',
        5,
        null,
        '{}'
    ),
    (
        9,
        3,
        '1.2.0',
        'sha256:logger-base-1.2.0-source',
        'sha256:logger-base-1.2.0-declarations',
        'Logging primitives and interfaces - Performance optimizations',
        'https://infra-tools.org/logger',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# logger-base\n\n## v1.2.0\n\nPerformance optimizations for high-throughput logging.',
        'https://github.com/infra-tools/logger-base',
        '{"logging", "observability", "infrastructure", "performance"}',
        now() - interval '20 days',
        5,
        null,
        '{}'
    );

-- INTERMEDIATE PACKAGE: cadence-apis (3 versions)
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        10,
        4,
        '0.0.1',
        'sha256:cadence-apis-0.0.1-source',
        'sha256:cadence-apis-0.0.1-declarations',
        'HTTP client/server APIs - Initial release',
        'https://cadence-org.dev/apis',
        'MIT OR Apache-2.0',
        '# Dual License: MIT OR Apache-2.0\n\nYou may choose either license.',
        '# cadence-apis\n\nHTTP client and server APIs built on cadence-core.',
        'https://github.com/cadence-org/cadence-apis',
        '{"http", "api", "client", "server"}',
        now() - interval '45 days',
        1,
        null,
        '{2}' -- depends on cadence-core@0.2.0 (version id 2)
    ),
    (
        11,
        4,
        '0.1.0',
        'sha256:cadence-apis-0.1.0-source',
        'sha256:cadence-apis-0.1.0-declarations',
        'HTTP client/server APIs - Updated to latest cadence-core',
        'https://cadence-org.dev/apis',
        'MIT OR Apache-2.0',
        '# Dual License: MIT OR Apache-2.0\n\nYou may choose either license.',
        '# cadence-apis\n\n## v0.1.0\n\nUpdated to use cadence-core 0.3.0 with concurrency support.',
        'https://github.com/cadence-org/cadence-apis',
        '{"http", "api", "client", "server", "async"}',
        now() - interval '20 days',
        1,
        null,
        '{3}' -- depends on cadence-core@0.3.0 (version id 3)
    ),
    (
        12,
        4,
        '0.1.1',
        'sha256:cadence-apis-0.1.1-source',
        'sha256:cadence-apis-0.1.1-declarations',
        'HTTP client/server APIs - Bugfix release',
        'https://cadence-org.dev/apis',
        'MIT OR Apache-2.0',
        '# Dual License: MIT OR Apache-2.0\n\nYou may choose either license.',
        '# cadence-apis\n\n## v0.1.1\n\nBugfix: Fixed connection pool leak in HTTP client.',
        'https://github.com/cadence-org/cadence-apis',
        '{"http", "api", "client", "server", "async"}',
        now() - interval '10 days',
        1,
        null,
        '{3}' -- depends on cadence-core@0.3.0 (version id 3)
    );

-- INTERMEDIATE PACKAGE: logger-fmt (2 versions)
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        13,
        5,
        '1.0.0',
        'sha256:logger-fmt-1.0.0-source',
        'sha256:logger-fmt-1.0.0-declarations',
        'Log formatting and output handlers - Stable release',
        'https://infra-tools.org/logger-fmt',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# logger-fmt\n\nLog formatting and output handlers for logger-base.',
        'https://github.com/infra-tools/logger-fmt',
        '{"logging", "formatting", "observability"}',
        now() - interval '40 days',
        5,
        null,
        '{7, 3}' -- depends on logger-base@1.0.0 (id 7), cadence-core@0.3.0 (id 3)
    ),
    (
        14,
        5,
        '1.1.0',
        'sha256:logger-fmt-1.1.0-source',
        'sha256:logger-fmt-1.1.0-declarations',
        'Log formatting and output handlers - Updated dependencies',
        'https://infra-tools.org/logger-fmt',
        'MIT',
        '# MIT License\n\nPermission is hereby granted...',
        '# logger-fmt\n\n## v1.1.0\n\nUpdated to logger-base 1.1.0 with structured logging.',
        'https://github.com/infra-tools/logger-fmt',
        '{"logging", "formatting", "observability", "structured"}',
        now() - interval '15 days',
        5,
        null,
        '{8, 3}' -- depends on logger-base@1.1.0 (id 8), cadence-core@0.3.0 (id 3)
    );

-- INTERMEDIATE PACKAGE: metrics-aggregator (3 versions)
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        15,
        6,
        '0.1.0',
        'sha256:metrics-aggregator-0.1.0-source',
        'sha256:metrics-aggregator-0.1.0-declarations',
        'Aggregates and processes metrics - Initial release',
        'https://metrics-labs.io/aggregator',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# metrics-aggregator\n\nAggregates and processes metrics from metrics-sdk.',
        'https://github.com/metrics-labs/metrics-aggregator',
        '{"metrics", "aggregation", "processing"}',
        now() - interval '35 days',
        4,
        null,
        '{4}' -- depends on metrics-sdk@0.1.0 (id 4)
    ),
    (
        16,
        6,
        '0.2.0',
        'sha256:metrics-aggregator-0.2.0-source',
        'sha256:metrics-aggregator-0.2.0-declarations',
        'Aggregates and processes metrics - Added histogram support',
        'https://metrics-labs.io/aggregator',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# metrics-aggregator\n\n## v0.2.0\n\nAdded histogram aggregation support.',
        'https://github.com/metrics-labs/metrics-aggregator',
        '{"metrics", "aggregation", "processing", "histograms"}',
        now() - interval '18 days',
        4,
        null,
        '{5}' -- depends on metrics-sdk@0.2.0 (id 5)
    ),
    (
        17,
        6,
        '0.2.1',
        'sha256:metrics-aggregator-0.2.1-source',
        'sha256:metrics-aggregator-0.2.1-declarations',
        'Aggregates and processes metrics - Bugfix release',
        'https://metrics-labs.io/aggregator',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# metrics-aggregator\n\n## v0.2.1\n\nBugfix: Updated to metrics-sdk 0.2.1.',
        'https://github.com/metrics-labs/metrics-aggregator',
        '{"metrics", "aggregation", "processing", "histograms"}',
        now() - interval '8 days',
        4,
        null,
        '{6}' -- depends on metrics-sdk@0.2.1 (id 6)
    );

-- LEAF PACKAGE: daisy-vec (2 versions)
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        18,
        7,
        '0.1.0',
        'sha256:daisy-vec-0.1.0-source',
        'sha256:daisy-vec-0.1.0-declarations',
        'Vector operations with metrics - Initial release',
        'https://daisy-ch.dev/vec',
        'GPL-3.0 OR MIT',
        '# Dual License: GPL-3.0 OR MIT\n\nYou may choose either license.',
        '# daisy-vec\n\nVector operations library with built-in metrics.',
        'https://github.com/daisy-ch/daisy-vec',
        '{"vector", "math", "metrics"}',
        now() - interval '12 days',
        2,
        null,
        '{11, 15}' -- depends on cadence-apis@0.1.0 (id 11), metrics-aggregator@0.1.0 (id 15)
    ),
    (
        19,
        7,
        '0.2.0',
        'sha256:daisy-vec-0.2.0-source',
        'sha256:daisy-vec-0.2.0-declarations',
        'Vector operations with metrics - Updated dependencies',
        'https://daisy-ch.dev/vec',
        'GPL-3.0 OR MIT',
        '# Dual License: GPL-3.0 OR MIT\n\nYou may choose either license.',
        '# daisy-vec\n\n## v0.2.0\n\nUpdated to latest dependencies with histogram support.',
        'https://github.com/daisy-ch/daisy-vec',
        '{"vector", "math", "metrics", "performance"}',
        now() - interval '5 days',
        2,
        null,
        '{12, 16}' -- depends on cadence-apis@0.1.1 (id 12), metrics-aggregator@0.2.0 (id 16)
    );

-- LEAF PACKAGE: ld-geo (2 versions) - DIAMOND CONVERGENCE
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        20,
        8,
        '0.1.0',
        'sha256:ld-geo-0.1.0-source',
        'sha256:ld-geo-0.1.0-declarations',
        'Geospatial utilities - Initial release (diamond: cadence-core via apis + logger-fmt)',
        'https://ld-corp.io/geo',
        'GPL-3.0 OR MIT',
        '# Dual License: GPL-3.0 OR MIT\n\nYou may choose either license.',
        '# ld-geo\n\nGeospatial utilities with logging and metrics.\n\nDemonstrates diamond dependency pattern.',
        'https://github.com/ld-corp/ld-geo',
        '{"geospatial", "mapping", "location"}',
        now() - interval '10 days',
        3,
        null,
        '{11, 13, 15}' -- depends on cadence-apis@0.1.0 (id 11), logger-fmt@1.0.0 (id 13), metrics-aggregator@0.1.0 (id 15)
    ),
    (
        21,
        8,
        '0.2.0',
        'sha256:ld-geo-0.2.0-source',
        'sha256:ld-geo-0.2.0-declarations',
        'Geospatial utilities - Updated to latest dependencies',
        'https://ld-corp.io/geo',
        'GPL-3.0 OR MIT',
        '# Dual License: GPL-3.0 OR MIT\n\nYou may choose either license.',
        '# ld-geo\n\n## v0.2.0\n\nUpdated to latest dependencies with improved performance.',
        'https://github.com/ld-corp/ld-geo',
        '{"geospatial", "mapping", "location", "performance"}',
        now() - interval '3 days',
        3,
        null,
        '{12, 14, 17}' -- depends on cadence-apis@0.1.1 (id 12), logger-fmt@1.1.0 (id 14), metrics-aggregator@0.2.1 (id 17)
    );

-- LEAF PACKAGE: observability-stack (2 versions) - LINEAR + TREE LEAF
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        22,
        9,
        '0.1.0',
        'sha256:observability-stack-0.1.0-source',
        'sha256:observability-stack-0.1.0-declarations',
        'Complete observability suite - Initial release (linear: logger-base → logger-fmt → this)',
        'https://infra-tools.org/observability',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# observability-stack\n\nComplete observability suite combining logging and metrics.\n\nDemonstrates linear dependency chain.',
        'https://github.com/infra-tools/observability-stack',
        '{"observability", "monitoring", "logging", "metrics"}',
        now() - interval '7 days',
        5,
        null,
        '{13, 16}' -- depends on logger-fmt@1.0.0 (id 13), metrics-aggregator@0.2.0 (id 16)
    ),
    (
        23,
        9,
        '0.2.0',
        'sha256:observability-stack-0.2.0-source',
        'sha256:observability-stack-0.2.0-declarations',
        'Complete observability suite - Updated dependencies',
        'https://infra-tools.org/observability',
        'Apache-2.0',
        '# Apache License 2.0\n\nLicensed under the Apache License...',
        '# observability-stack\n\n## v0.2.0\n\nUpdated to latest logging and metrics libraries.',
        'https://github.com/infra-tools/observability-stack',
        '{"observability", "monitoring", "logging", "metrics", "structured"}',
        now() - interval '2 days',
        5,
        null,
        '{14, 17}' -- depends on logger-fmt@1.1.0 (id 14), metrics-aggregator@0.2.1 (id 17)
    );

-- LEAF PACKAGE: app-framework (2 versions) - MULTI-CONVERGENCE
insert into
    version(
        id,
        package,
        qualified_version,
        source_checksum,
        declarations_checksum,
        description,
        homepage,
        license,
        license_text,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id,
        dependencies
    )
values
    (
        24,
        10,
        '0.1.0',
        'sha256:app-framework-0.1.0-source',
        'sha256:app-framework-0.1.0-declarations',
        'Full application framework - Initial release (converges multiple dependency trees)',
        'https://cadence-org.dev/framework',
        'MIT OR Apache-2.0',
        '# Dual License: MIT OR Apache-2.0\n\nYou may choose either license.',
        '# app-framework\n\nFull application framework combining APIs, metrics, logging, and vector operations.\n\nDemonstrates multi-pattern convergence.',
        'https://github.com/cadence-org/app-framework',
        '{"framework", "application", "full-stack"}',
        now() - interval '4 days',
        1,
        null,
        '{11, 16, 13, 18}' -- depends on cadence-apis@0.1.0 (id 11), metrics-aggregator@0.2.0 (id 16), logger-fmt@1.0.0 (id 13), daisy-vec@0.1.0 (id 18)
    ),
    (
        25,
        10,
        '0.2.0',
        'sha256:app-framework-0.2.0-source',
        'sha256:app-framework-0.2.0-declarations',
        'Full application framework - Updated to latest dependencies',
        'https://cadence-org.dev/framework',
        'MIT OR Apache-2.0',
        '# Dual License: MIT OR Apache-2.0\n\nYou may choose either license.',
        '# app-framework\n\n## v0.2.0\n\nUpdated to latest versions of all dependencies.',
        'https://github.com/cadence-org/app-framework',
        '{"framework", "application", "full-stack", "performance"}',
        now() - interval '1 day',
        1,
        null,
        '{12, 17, 14, 19}' -- depends on cadence-apis@0.1.1 (id 12), metrics-aggregator@0.2.1 (id 17), logger-fmt@1.1.0 (id 14), daisy-vec@0.2.0 (id 19)
    );

select
    setval('version_id_seq', 26);

-- ============================================================================
-- SCHEMA ADMINS
-- ============================================================================
insert into
    schema_admin (id, package, org_id, user_id, revoked_at)
values
    (1, 1, 1, null, null),
    -- cadence-core: cadence-org
    (2, 2, 4, null, null),
    -- metrics-sdk: metrics-labs
    (3, 3, 5, null, null),
    -- logger-base: infra-tools
    (4, 4, 1, null, null),
    -- cadence-apis: cadence-org
    (5, 5, 5, null, null),
    -- logger-fmt: infra-tools
    (6, 6, 4, null, null),
    -- metrics-aggregator: metrics-labs
    (7, 7, 2, null, null),
    -- daisy-vec: daisy-ch
    (8, 8, 3, null, null),
    -- ld-geo: ld-corp
    (9, 9, 5, null, null),
    -- observability-stack: infra-tools
    (10, 10, 1, null, null);

-- app-framework: cadence-org
select
    setval('schema_admin_id_seq', 11);