#!/usr/bin/env python3
"""605-db-vilorm-crud — Python SDK equivalent
Compile: vil compile --from python --input 605-db-vilorm-crud.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("vilorm-showcase", port=8080)
blog = server.service_process("blog")
blog.endpoint("POST", "/authors", "create_author")
blog.endpoint("GET", "/authors", "list_authors")
blog.endpoint("GET", "/posts", "list_posts")
blog.endpoint("POST", "/posts", "create_post")
blog.endpoint("GET", "/posts/:id", "get_post")
blog.endpoint("PUT", "/posts/:id", "update_post")
blog.endpoint("DELETE", "/posts/:id", "delete_post")
blog.endpoint("POST", "/tags", "create_tag")
blog.endpoint("GET", "/stats", "blog_stats")
server.compile()
