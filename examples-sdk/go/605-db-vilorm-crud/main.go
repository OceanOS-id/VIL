// 605 — Blog Platform (VilORM CRUD)
// Equivalent to: examples/605-db-vilorm-crud (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	server := vil.NewServer("vilorm-showcase", 8080)

	blog := vil.NewService("blog")
	blog.Endpoint("POST", "/authors", "create_author")
	blog.Endpoint("GET", "/authors", "list_authors")
	blog.Endpoint("GET", "/posts", "list_posts")
	blog.Endpoint("POST", "/posts", "create_post")
	blog.Endpoint("GET", "/posts/:id", "get_post")
	blog.Endpoint("PUT", "/posts/:id", "update_post")
	blog.Endpoint("DELETE", "/posts/:id", "delete_post")
	blog.Endpoint("POST", "/tags", "create_tag")
	blog.Endpoint("GET", "/stats", "blog_stats")
	server.Service(blog)

	server.Compile()
}
