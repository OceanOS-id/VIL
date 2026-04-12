// Dynamic Pricing Engine — AssemblyScript WASM
// Compile: asc pricing.ts -o pricing.wasm --runtime stub

import { JSON } from "json-as";

export function calculate(inputJson: string): string {
  const input = JSON.parse(inputJson);
  const basePrice: f64 = parseFloat(input.getString("base_price") || "100");
  const quantity: i32 = parseInt(input.getString("quantity") || "1") as i32;
  const tier: string = input.getString("tier") || "standard";

  let discount: f64 = 0;
  if (tier == "premium") discount = 0.15;
  else if (tier == "enterprise") discount = 0.25;
  else if (quantity > 100) discount = 0.10;
  else if (quantity > 10) discount = 0.05;

  const subtotal = basePrice * quantity;
  const total = subtotal * (1.0 - discount);

  return `{"base_price":${basePrice},"quantity":${quantity},"tier":"${tier}","discount":${discount},"subtotal":${subtotal},"total":${total}}`;
}
