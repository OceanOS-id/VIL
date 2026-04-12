/**
 * Payment Processor — Java WASM Module
 * Luhn card validation + charge processing for payment gateway.
 */

public class PaymentProcessor {
    
    static boolean luhnCheck(String cardNumber) {
        String digits = cardNumber.replaceAll("[^0-9]", "");
        if (digits.length() < 13) return false;
        int sum = 0;
        boolean alternate = false;
        for (int i = digits.length() - 1; i >= 0; i--) {
            int n = digits.charAt(i) - '0';
            if (alternate) { n *= 2; if (n > 9) n -= 9; }
            sum += n;
            alternate = !alternate;
        }
        return sum % 10 == 0;
    }
    
    public static void main(String[] args) throws Exception {
        java.io.BufferedReader reader = new java.io.BufferedReader(new java.io.InputStreamReader(System.in));
        StringBuilder sb = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) sb.append(line);
        String input = sb.toString();
        
        String op = extractField(input, "operation");
        if (op.isEmpty()) op = "validate";
        
        if (op.equals("validate")) {
            String card = extractField(input, "card_number");
            boolean valid = luhnCheck(card);
            String last4 = card.length() >= 4 ? card.substring(card.length() - 4) : "????";
            System.out.printf("{\"valid\":%s,\"card_last4\":\"%s\"}%n", valid, last4);
        } else if (op.equals("charge")) {
            double amount = extractDouble(input, "amount", 0);
            String currency = extractField(input, "currency");
            if (currency.isEmpty()) currency = "USD";
            String paymentId = "pay_" + Long.toHexString(System.currentTimeMillis());
            System.out.printf("{\"payment_id\":\"%s\",\"amount\":%.2f,\"currency\":\"%s\",\"status\":\"charged\"}%n",
                paymentId, amount, currency);
        }
    }
    
    static String extractField(String json, String key) {
        String p = "\"" + key + "\":\"";
        int pos = json.indexOf(p);
        if (pos < 0) return "";
        int s = pos + p.length();
        int e = json.indexOf("\"", s);
        return e > s ? json.substring(s, e) : "";
    }
    
    static double extractDouble(String json, String key, double def) {
        String p = "\"" + key + "\":";
        int pos = json.indexOf(p);
        if (pos < 0) return def;
        int s = pos + p.length();
        StringBuilder num = new StringBuilder();
        for (int i = s; i < json.length(); i++) {
            char c = json.charAt(i);
            if (Character.isDigit(c) || c == '.' || c == '-') num.append(c);
            else if (num.length() > 0) break;
        }
        try { return Double.parseDouble(num.toString()); }
        catch (Exception e) { return def; }
    }
}
