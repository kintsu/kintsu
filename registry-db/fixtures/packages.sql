insert into
    org (id, name, gh_id, gh_avatar)
values
    (
        2,
        'example-org',
        123456,
        'https://avatars.githubusercontent.com/u/123456?v=4'
    ),
    (
        3,
        'another-org',
        789012,
        'https://avatars.githubusercontent.com/u/789012?v=4'
    );

insert into
    org_admin (org_id, user_id, revoked_at)
values
    (2, 1, null),
    (3, 1, null);

insert into
    package (id, name)
values
    (1, 'example-org-package'),
    (2, 'another-org-package'),
    (3, 'personal-package');

insert into
    version(
        id,
        package,
        qualified_version,
        checksum,
        description,
        homepage,
        license,
        readme,
        repository,
        keywords,
        created_at,
        publishing_org_id,
        publishing_user_id
    )
values
    (
        1,
        1,
        '1.0.0',
        'checksum1',
        'An example package for example-org',
        'https://example.org',
        '# MIT License\n\nMIT License',
        '# This is the readme for example-org-package.',
        'https://github.com/example-org/example-org-package',
        '{"example", "org", "package"}',
        now(),
        2,
        null
    ),
    (
        2,
        2,
        '0.1.0',
        'checksum2',
        'A package for another-org',
        'https://another.org',
        '# Apache License 2.0\n\nApache License 2.0',
        '# This is the readme for another-org-package.',
        'https://github.com/another-org/another-org-package',
        '{"another", "org", "package"}',
        now(),
        3,
        null
    ),
    (
        3,
        3,
        '0.0.1',
        'checksum3',
        'A personal package by user 1',
        'https://personal.example.com',
        '# GPL-3.0\n\nGPL-3.0',
        '# This is the readme for personal-package.',
        'https://github.com/user1/personal-package',
        '{"personal", "package"}',
        now(),
        null,
        1
    );

insert into
    schema_admin (package, org_id, user_id, revoked_at)
values
    (1, 2, null, null),
    (2, 3, null, null),
    (3, null, 1, null);