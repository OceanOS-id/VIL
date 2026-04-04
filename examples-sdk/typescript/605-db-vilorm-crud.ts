#!/usr/bin/env tsx
// 605-db-vilorm-crud — TypeScript SDK equivalent
// Compile: vil compile --from typescript --input 605-db-vilorm-crud.ts --release

import { VilPipeline, VilServer, ServiceProcess } from "vil-sdk";

const server = new VilServer("vilorm-showcase", 8080);
const blog = new ServiceProcess("blog");
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
