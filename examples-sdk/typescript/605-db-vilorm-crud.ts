/**
 * 605 — Blog Platform (VilORM CRUD)
 * Declares Author + Post entities; vil compile generates VilORM-backed Rust.
 * Compile: vil compile --from typescript --input 605-db-vilorm-crud.ts --release
 */
import { VilServer, ServiceProcess } from 'vil-sdk';

const server = new VilServer('blog-platform', 8080);

// -- Entities (semantic_type kind="entity" -> VilORM codegen) ----------------
server.semanticType('Author', 'entity', {
  id: 'String', name: 'String', bio: 'String', posts_count: 'u64',
});
server.semanticType('Post', 'entity', {
  id: 'String', title: 'String', body: 'String',
  author_id: 'String', published: 'bool',
});

// -- ServiceProcess: blog ----------------------------------------------------
const blog = new ServiceProcess('blog');
blog.endpoint('GET',    '/authors',     'list_authors');
blog.endpoint('GET',    '/authors/:id', 'get_author');
blog.endpoint('POST',   '/authors',     'create_author');
blog.endpoint('PUT',    '/authors/:id', 'update_author');
blog.endpoint('DELETE', '/authors/:id', 'delete_author');
blog.endpoint('GET',    '/posts',       'list_posts');
blog.endpoint('GET',    '/posts/:id',   'get_post');
blog.endpoint('POST',   '/posts',       'create_post');
blog.endpoint('PUT',    '/posts/:id',   'update_post');
blog.endpoint('DELETE', '/posts/:id',   'delete_post');
server.service(blog, '/api');

// -- Emit / compile ----------------------------------------------------------
server.compile();
