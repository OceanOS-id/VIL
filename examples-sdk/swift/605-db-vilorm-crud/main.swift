// 605-db-vilorm-crud — Swift SDK equivalent
// Compile: vil compile --from swift --input 605-db-vilorm-crud/main.swift --release

let server = VilServer(name: "vilorm-showcase", port: 8080)
let blog = ServiceProcess(name: "blog")
blog.endpoint(method: "POST", path: "/authors", handler: "create_author")
blog.endpoint(method: "GET", path: "/authors", handler: "list_authors")
blog.endpoint(method: "GET", path: "/posts", handler: "list_posts")
blog.endpoint(method: "POST", path: "/posts", handler: "create_post")
blog.endpoint(method: "GET", path: "/posts/:id", handler: "get_post")
blog.endpoint(method: "PUT", path: "/posts/:id", handler: "update_post")
blog.endpoint(method: "DELETE", path: "/posts/:id", handler: "delete_post")
blog.endpoint(method: "POST", path: "/tags", handler: "create_tag")
blog.endpoint(method: "GET", path: "/stats", handler: "blog_stats")
server.service(blog)
server.compile()
