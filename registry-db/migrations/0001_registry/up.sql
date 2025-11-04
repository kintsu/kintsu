create table package (
    id bigserial primary key not null,
    name varchar(128) not null
);

comment on table package is 'A package represents a collection of schema versions identified by a unique name.';

create unique index package_name_idx on package(name);

/*
 -------------------
 */
create table users (
    id bigserial primary key not null,
    email varchar not null,
    gh_id int not null,
    gh_login varchar not null,
    gh_avatar varchar
);

comment on table users is 'A user represents an individual account with access to the registry (only github for now).';

create unique index users_gh_id_idx on users(gh_id);

create unique index user_email_idx on users(email);

/*
 -------------------
 */
create table org (
    id bigserial primary key not null,
    name varchar not null,
    gh_id int not null
);

comment on table org is 'An organization represents a group of users that can own and administer packages (from github).';

create unique index org_name_idx on org(name);

create unique index org_gh_id_idx on org(gh_id);

/*
 -------------------
 */
create table org_admin (
    org_id bigint not null references org(id),
    user_id bigint not null references users(id),
    revoked_at timestamptz,
    primary key (org_id, user_id)
);

comment on table org_admin is 'An org_admin represents a user who has administrative privileges for a specific organization.';

/*
 -------------------
 */
create table schema_admin (
    id bigserial primary key not null,
    package bigint not null references package(id),
    user_id bigint references users(id),
    org_id bigint references org(id),
    revoked_at timestamptz,
    check (
        (
            user_id is null
            and org_id <> null
        )
        or (
            org_id is null
            and user_id <> null
        )
    )
);

comment on table schema_admin is 'A schema_admin represents an administrator for a specific package, which can be either a user or an organization.';

create unique index schema_user_admins_idx on schema_admin(package, user_id)
where
    user_id <> null;

create unique index schema_org_admins_idx on schema_admin(package, org_id)
where
    org_id <> null;

/*
 -------------------
 */
create table api_key (
    id bigserial primary key not null,
    key bytea not null,
    description varchar(32),
    expires timestamptz not null,
    scopes text [] not null,
    permissions text [] not null,
    user_id bigint references users(id),
    org_id bigint references org(id),
    last_used_at timestamptz,
    revoked_at timestamptz,
    check (
        (
            user_id is null
            and org_id <> null
        )
        or (
            org_id is null
            and user_id <> null
        )
    )
);

comment on table api_key is 'An api key represents a personal access token for a user to authenticate API requests.';

/*
 -------------------
 */
create table version (
    id bigserial primary key not null,
    package bigint not null references package(id),
    qualified_version varchar(32) not null,
    checksum varchar(255) not null,
    description varchar(1024),
    homepage varchar(128),
    license varchar not null,
    readme varchar not null,
    repository varchar not null,
    keywords text [] not null default '{}',
    created_at timestamptz not null default now(),
    yanked_at timestamptz,
    publishing_org_id bigint references org(id),
    publishing_user_id bigint references users(id),
    check (
        (
            publishing_user_id is null
            and publishing_org_id <> null
        )
        or (
            publishing_org_id is null
            and publishing_user_id <> null
        )
    )
);

comment on table version is 'A version represents a specific release of a package, including metadata such as checksum, description, license, and publisher information.';

comment on column version.qualified_version is 'The qualified version follows semantic versioning (semver) format, e.g., "1.0.0".';

create unique index package_version_idx on version (package, qualified_version);

/*
 -------------------
 */
create table downloads (
    version bigint not null references version (id),
    day date not null,
    count int not null default '0',
    primary key (version, day)
);

comment on table downloads is 'A downloads record represents the number of times a specific version of a package was downloaded on a particular day.';

/*
 -------------------
 - schema
 - version
 - user
 - org
 - schema_admin
 - downloads
 
 - admin_invitations (future)
 */