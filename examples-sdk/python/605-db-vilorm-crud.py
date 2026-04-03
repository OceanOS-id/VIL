#!/usr/bin/env python3
"""605 — Blog Platform (VilORM CRUD)
Declares Author + Post entities; vil compile generates VilORM-backed Rust.
Compile: vil compile --from python --input 605-db-vilorm-crud.py --release
"""
import os
from vil import VilServer

server = VilServer("blog-platform", port=8080)

# -- Entities (semantic_type kind="entity" -> VilORM codegen) ----------------
server.semantic_type("Author", "entity", fields={
    "id": "String", "name": "String", "bio": "String", "posts_count": "u64",
})
server.semantic_type("Post", "entity", fields={
    "id": "String", "title": "String", "body": "String",
    "author_id": "String", "published": "bool",
})

# -- ServiceProcess: blog ----------------------------------------------------
blog = server.service_process("blog", prefix="/api")
blog.endpoint("GET",    "/authors",     "list_authors")
blog.endpoint("GET",    "/authors/:id", "get_author")
blog.endpoint("POST",   "/authors",     "create_author")
blog.endpoint("PUT",    "/authors/:id", "update_author")
blog.endpoint("DELETE", "/authors/:id", "delete_author")
blog.endpoint("GET",    "/posts",       "list_posts")
blog.endpoint("GET",    "/posts/:id",   "get_post")
blog.endpoint("POST",   "/posts",       "create_post")
blog.endpoint("PUT",    "/posts/:id",   "update_post")
blog.endpoint("DELETE", "/posts/:id",   "delete_post")

# -- Emit / compile ----------------------------------------------------------
if os.environ.get("VIL_COMPILE_MODE") == "manifest":
    print(server.to_yaml())
else:
    server.compile()
