// Product Query — AssemblyScript WASM
// Simple product catalog filter by category.

export function query(inputJson: string): string {
  // Mock product catalog
  const products = [
    '{"id":1,"name":"Widget A","category":"electronics","price":29.99}',
    '{"id":2,"name":"Gadget B","category":"electronics","price":49.99}',
    '{"id":3,"name":"Book C","category":"books","price":12.99}',
    '{"id":4,"name":"Tool D","category":"hardware","price":34.50}'
  ];

  // Extract category filter (simple string search)
  let category = "";
  const catIdx = inputJson.indexOf('"category":"');
  if (catIdx >= 0) {
    const start = catIdx + 12;
    const end = inputJson.indexOf('"', start);
    if (end > start) category = inputJson.substring(start, end);
  }

  let result = "[";
  let count = 0;
  for (let i = 0; i < products.length; i++) {
    if (category == "" || products[i].includes('"' + category + '"')) {
      if (count > 0) result += ",";
      result += products[i];
      count++;
    }
  }
  result += "]";

  return `{"products":${result},"total":${count},"filter":"${category}"}`;
}
