create table registry (
    name varchar not null primary key,
    url varchar not null
);

create table package (
    registry varchar not null references registry(name),
    name varchar not null,
    version varchar not null,
    content_hash varchar not null,
    content varchar not null,
    primary key (registry, name, version)
);
