/**
 * Banking Transfer — Java WASM Module
 * Validates and processes bank transfers with structured domain errors.
 */

public class BankingTransfer {
    
    public static void main(String[] args) throws Exception {
        java.io.BufferedReader reader = new java.io.BufferedReader(new java.io.InputStreamReader(System.in));
        StringBuilder sb = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) sb.append(line);
        String input = sb.toString();
        
        double amount = extractDouble(input, "amount", 0);
        String fromAccount = extractField(input, "from_account");
        String toAccount = extractField(input, "to_account");
        double balance = extractDouble(input, "balance", 1000000);
        
        // Validation
        if (amount <= 0) {
            System.out.println("{\"error\":\"INVALID_AMOUNT\",\"message\":\"Amount must be positive\"}");
            return;
        }
        if (amount > balance) {
            System.out.printf("{\"error\":\"INSUFFICIENT_FUNDS\",\"message\":\"Balance %.2f < amount %.2f\"}%n", balance, amount);
            return;
        }
        if (fromAccount.equals(toAccount)) {
            System.out.println("{\"error\":\"SAME_ACCOUNT\",\"message\":\"Cannot transfer to same account\"}");
            return;
        }
        
        String txId = "TXN-" + Long.toHexString(System.currentTimeMillis()).toUpperCase();
        double newBalance = balance - amount;
        System.out.printf("{\"tx_id\":\"%s\",\"from\":\"%s\",\"to\":\"%s\",\"amount\":%.2f,\"new_balance\":%.2f,\"status\":\"completed\"}%n",
            txId, fromAccount, toAccount, amount, newBalance);
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
