// Embedded GraphiQL playground HTML.

use axum::response::Html;

/// Serve the GraphiQL IDE at /graphql/playground.
pub async fn graphiql_handler() -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>VIL GraphQL Playground</title>
    <style>body {{ height: 100vh; margin: 0; }}</style>
    <link rel="stylesheet" href="https://unpkg.com/graphiql@3/graphiql.min.css" />
</head>
<body>
    <div id="graphiql" style="height: 100vh;"></div>
    <script crossorigin src="https://unpkg.com/react@18/umd/react.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/graphiql@3/graphiql.min.js"></script>
    <script>
        const fetcher = GraphiQL.createFetcher({{ url: '/graphql' }});
        ReactDOM.createRoot(document.getElementById('graphiql')).render(
            React.createElement(GraphiQL, {{ fetcher }})
        );
    </script>
</body>
</html>"#
    ))
}
