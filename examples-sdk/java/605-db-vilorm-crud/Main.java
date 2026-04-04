// 605-db-vilorm-crud — Java SDK equivalent
// Compile: vil compile --from java --input 605-db-vilorm-crud/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("vilorm-showcase", 8080);
        ServiceProcess blog = new ServiceProcess("blog");
        blog.endpoint("POST", "/authors", "create_author");
        blog.endpoint("GET", "/authors", "list_authors");
        blog.endpoint("GET", "/posts", "list_posts");
        blog.endpoint("POST", "/posts", "create_post");
        blog.endpoint("GET", "/posts/:id", "get_post");
        blog.endpoint("PUT", "/posts/:id", "update_post");
        blog.endpoint("DELETE", "/posts/:id", "delete_post");
        blog.endpoint("POST", "/tags", "create_tag");
        blog.endpoint("GET", "/stats", "blog_stats");
        server.service(blog);
        server.compile();
    }
}
