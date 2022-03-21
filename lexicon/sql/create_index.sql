create table lexindex (
    lexcode     int     not null    primary key,
    name        text    not null,
    lang        text    not null,
    size        int     not null,
    brief       text,
    author      text,
    version     text,
    tags        text,
    usedtimes   int
);

