// Restaurant Order — AssemblyScript WASM
// Menu lookup + order total calculation.

export function processOrder(inputJson: string): string {
  // Menu prices
  const menuPrices: Map<string, f64> = new Map();
  menuPrices.set("nasi_goreng", 25000);
  menuPrices.set("mie_ayam", 20000);
  menuPrices.set("sate_ayam", 35000);
  menuPrices.set("es_teh", 5000);
  menuPrices.set("es_jeruk", 8000);

  // Simple parse items array
  let total: f64 = 0;
  let itemCount: i32 = 0;

  for (let i = 0; i < menuPrices.keys().length; i++) {
    const key = menuPrices.keys()[i];
    if (inputJson.includes(key)) {
      total += menuPrices.get(key);
      itemCount++;
    }
  }

  if (itemCount == 0) {
    total = 25000; // default: nasi goreng
    itemCount = 1;
  }

  return `{"items":${itemCount},"subtotal":${total},"tax":${total * 0.1},"total":${total * 1.1},"currency":"IDR"}`;
}
