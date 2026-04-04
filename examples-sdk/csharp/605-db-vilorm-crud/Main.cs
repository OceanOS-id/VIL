// 605-db-vilorm-crud — C# SDK equivalent
// Compile: vil compile --from csharp --input 605-db-vilorm-crud/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("vilorm-showcase", 8080);
var blog = new ServiceProcess("blog");
blog.Endpoint("POST", "/authors", "create_author");
blog.Endpoint("GET", "/authors", "list_authors");
blog.Endpoint("GET", "/posts", "list_posts");
blog.Endpoint("POST", "/posts", "create_post");
blog.Endpoint("GET", "/posts/:id", "get_post");
blog.Endpoint("PUT", "/posts/:id", "update_post");
blog.Endpoint("DELETE", "/posts/:id", "delete_post");
blog.Endpoint("POST", "/tags", "create_tag");
blog.Endpoint("GET", "/stats", "blog_stats");
server.Service(blog);
server.Compile();
