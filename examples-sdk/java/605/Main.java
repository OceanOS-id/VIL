/**
 * 605 — Blog Platform (VilORM CRUD)
 * Declares Author + Post entities; vil compile generates VilORM-backed Rust.
 * Compile: vil compile --from java --input 605/Main.java --release
 */
package dev.vil.examples;

import dev.vil.VilServer;
import dev.vil.ServiceProcess;
import java.util.LinkedHashMap;
import java.util.Map;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("blog-platform", 8080);

        // -- Entities (semantic_type kind="entity" -> VilORM codegen) --------
        server.semanticType("Author", "entity", fields(
            "id", "String", "name", "String", "bio", "String", "posts_count", "u64"
        ), null);
        server.semanticType("Post", "entity", fields(
            "id", "String", "title", "String", "body", "String",
            "author_id", "String", "published", "bool"
        ), null);

        // -- ServiceProcess: blog --------------------------------------------
        ServiceProcess blog = new ServiceProcess("blog");
        blog.endpoint("GET",    "/authors",     "list_authors");
        blog.endpoint("GET",    "/authors/:id", "get_author");
        blog.endpoint("POST",   "/authors",     "create_author");
        blog.endpoint("PUT",    "/authors/:id", "update_author");
        blog.endpoint("DELETE", "/authors/:id", "delete_author");
        blog.endpoint("GET",    "/posts",       "list_posts");
        blog.endpoint("GET",    "/posts/:id",   "get_post");
        blog.endpoint("POST",   "/posts",       "create_post");
        blog.endpoint("PUT",    "/posts/:id",   "update_post");
        blog.endpoint("DELETE", "/posts/:id",   "delete_post");
        server.service(blog);

        // -- Emit / compile --------------------------------------------------
        server.compile(true);
    }

    private static Map<String, String> fields(String... pairs) {
        Map<String, String> m = new LinkedHashMap<>();
        for (int i = 0; i < pairs.length; i += 2) m.put(pairs[i], pairs[i + 1]);
        return m;
    }
}
