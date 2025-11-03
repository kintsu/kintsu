create table package (
    id bigserial primary key not null,
    name varchar(128) not null,
    description varchar(1024) not null
);

create unique index package_name_idx on package(name);

create table package_version (
    id bigserial primary key not null,
    package bigint references package(id) not null,
    -- 32 chars should be enough for semantic versioning?
    version varchar(32) not null,
    checksum varchar(128) not null,
    metadata jsonb not null,
    -- package_name + version + 1 char = 128 + 32 + 1 = 161 < 256
    s3_key varchar(256) not null,
    --
    created_at timestamptz not null default now(),
    yanked_at timestamptz,
    unique(package_name, version)
);

create index idx_versions_metadata_keywords ON package_version USING gin((metadata -> 'keywords'));

create table downloads (
    day date not null,
    version bigint references package_version(id) not null,
    -- highly unlikely to exceed 2^31 in a day
    count int not null default 0,
    primary key(day, version)
);