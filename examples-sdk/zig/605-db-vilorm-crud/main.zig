// 605-db-vilorm-crud — Zig SDK equivalent
// Compile: vil compile --from zig --input 605-db-vilorm-crud/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("vilorm-showcase", 8080);
    var blog = vil.Service.init("blog");
    blog.endpoint("POST", "/authors", "create_author");
    blog.endpoint("GET", "/authors", "list_authors");
    blog.endpoint("GET", "/posts", "list_posts");
    blog.endpoint("POST", "/posts", "create_post");
    blog.endpoint("GET", "/posts/:id", "get_post");
    blog.endpoint("PUT", "/posts/:id", "update_post");
    blog.endpoint("DELETE", "/posts/:id", "delete_post");
    blog.endpoint("POST", "/tags", "create_tag");
    blog.endpoint("GET", "/stats", "blog_stats");
    server.service(&blog);
    server.compile();
}
