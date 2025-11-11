create table package (
    id bigserial primary key not null,
    name varchar(64) not null
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
    gh_id int not null,
    gh_avatar varchar not null
);

comment on table org is 'An organization represents a group of users that can own and administer packages (from github).';

create unique index org_name_idx on org(name);

create unique index org_gh_id_idx on org(gh_id);

/*
 -------------------
 */
create type org_role_type as enum ('admin', 'member');

create table org_role (
    org_id bigint not null references org(id),
    user_id bigint not null references users(id),
    role org_role_type not null,
    revoked_at timestamptz,
    primary key (org_id, user_id)
);

comment on table org_role is 'An org_role represents a role for a specific organization.';

/*
 -------------------
 */
create type schema_role_type as enum (
    'admin',
    -- author is specifically for publishing rights when named in a manifest
    -- there are no special rights associated with this role outside attributions
    'author'
);

create table schema_role (
    id bigserial primary key not null,
    package bigint not null references package(id),
    user_id bigint references users(id),
    org_id bigint references org(id),
    role schema_role_type not null,
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

comment on table schema_role is 'A schema_role represents a role for a specific package, which can be either a user or an organization.';

create unique index schema_user_roles_idx on schema_role(package, user_id)
where
    user_id <> null;

create unique index schema_org_roles_idx on schema_role(package, org_id)
where
    org_id <> null;

/*
 -------------------
 */
create type permission as enum (
    'publish-package',
    'yank-package',
    'grant-organization-role',
    'grant-schema-role'
);

create table api_key (
    id bigserial primary key not null,
    key bytea not null,
    description varchar(32),
    expires timestamptz not null,
    scopes varchar(64) [] not null,
    permissions permission [] not null,
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
    source_checksum varchar(255) not null,
    declarations_checksum varchar(255) not null,
    -- source_encoding
    description varchar(1024),
    homepage varchar(128),
    -- SPDX license expression
    license varchar(128) not null,
    -- markdown readme content
    license_text varchar not null,
    readme varchar not null,
    repository varchar not null,
    -- self-referential foreign key to other versions this version depends on
    -- since we do not allow deletions, this is safe.
    dependencies bigint [] not null,
    keywords text [] not null default '{}',
    created_at timestamptz not null default now(),
    yanked_at timestamptz,
    -- -- declarations per parser
    -- declarations jsonb not null,
    -- -- raw source jsonb of { "./path/to/file": "file content", ... }
    -- source jsonb not null,
    -- publisher
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
 */
create table user_favourite (
    id bigint primary key,
    user_id bigint not null references users(id),
    package_id bigint references package(id),
    org_id bigint references org(id),
    check (
        (
            package_id is null
            and org_id <> null
        )
        or (
            org_id is null
            and package_id <> null
        )
    )
);

create unique index user_package_favourite_idx on user_favourite(user_id, package_id)
where
    package_id <> null;

create unique index user_org_favourite_idx on user_favourite(user_id, org_id)
where
    org_id <> null;

comment on table user_favourite is 'A user_favourite represents a package that a user has marked as a favorite.';

/*
 -------------------
 */
create table org_invitation (
    id bigserial primary key not null,
    org_id bigint not null references org(id),
    inviting_user_id bigint not null references users(id),
    invited_user_gh_login varchar not null,
    role org_role_type not null,
    created_at timestamptz not null default now(),
    accepted_at timestamptz,
    revoked_at timestamptz
);

/*
 -------------------
 - schema
 - version
 - user
 - org
 - schema_admin
 - downloads

 - org invitations (future)
 */
create
or replace function get_dependency_tree(root_version_ids bigint []) returns table (
    version_id bigint,
    package_id bigint,
    level int,
    path bigint [],
    direct_dependency_count int
) language sql stable as $$ with recursive dependency_tree as (
    select
        v.id as version_id,
        v.package as package_id,
        v.dependencies,
        0 as level,
        array [v.id] as path
    from
        version v
    where
        v.id = any(root_version_ids)
    union
    all
    select
        v.id as version_id,
        v.package as package_id,
        v.dependencies,
        dt.level + 1 as level,
        dt.path || v.id as path
    from
        dependency_tree dt
        cross join lateral unnest(dt.dependencies) as dep_id
        join version v on v.id = dep_id
    where
        not (v.id = any(dt.path))
        and dt.level < 1000
)
select
    dt.version_id,
    dt.package_id,
    dt.level,
    dt.path,
    coalesce(array_length(dt.dependencies, 1), 0) as direct_dependency_count
from
    dependency_tree dt
order by
    dt.level,
    dt.version_id $$;