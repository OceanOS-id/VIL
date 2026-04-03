// 605 — Blog Platform (VilORM CRUD)
// Declares Author + Post entities; vil compile generates VilORM-backed Rust.
// Compile: vil compile --from go --input 605/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	server := vil.NewServer("blog-platform", 8080)

	// -- Entities (semantic_type kind="entity" -> VilORM codegen) ------------
	server.SemanticType("Author", "entity", map[string]string{
		"id": "String", "name": "String", "bio": "String", "posts_count": "u64",
	}, nil)
	server.SemanticType("Post", "entity", map[string]string{
		"id": "String", "title": "String", "body": "String",
		"author_id": "String", "published": "bool",
	}, nil)

	// -- ServiceProcess: blog ------------------------------------------------
	blog := vil.NewService("blog")
	blog.Endpoint("GET", "/authors", "list_authors")
	blog.Endpoint("GET", "/authors/:id", "get_author")
	blog.Endpoint("POST", "/authors", "create_author")
	blog.Endpoint("PUT", "/authors/:id", "update_author")
	blog.Endpoint("DELETE", "/authors/:id", "delete_author")
	blog.Endpoint("GET", "/posts", "list_posts")
	blog.Endpoint("GET", "/posts/:id", "get_post")
	blog.Endpoint("POST", "/posts", "create_post")
	blog.Endpoint("PUT", "/posts/:id", "update_post")
	blog.Endpoint("DELETE", "/posts/:id", "delete_post")
	server.Service(blog)

	// -- Emit / compile ------------------------------------------------------
	server.Compile()
}
