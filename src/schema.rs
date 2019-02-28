table! {
    authors (id) {
        id -> Int4,
        name -> Varchar,
        email -> Varchar,
        visible -> Bool,
    }
}

table! {
    commits (sha) {
        sha -> Varchar,
        release_id -> Int4,
        author_id -> Int4,
    }
}

table! {
    maintenances (id) {
        id -> Int4,
        enabled -> Bool,
    }
}

table! {
    projects (id) {
        id -> Int4,
        name -> Varchar,
        url_path -> Varchar,
        github_name -> Varchar,
    }
}

table! {
    releases (id) {
        id -> Int4,
        version -> Varchar,
        project_id -> Int4,
        visible -> Bool,
        link -> Varchar,
    }
}

joinable!(commits -> authors (author_id));
joinable!(releases -> projects (project_id));

allow_tables_to_appear_in_same_query!(
    authors,
    commits,
    maintenances,
    projects,
    releases,
);