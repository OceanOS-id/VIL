#!/usr/bin/env ruby
# NPL Account Flagger — Ruby Sidecar
require 'json'
input = JSON.parse(STDIN.read rescue '{}')
kol = input['kolektabilitas'].to_i
outstanding = input['outstanding'].to_f
days = input['days_overdue'].to_i
is_npl = kol >= 3 || days > 90
severity = case kol
  when 5.. then 'critical'
  when 4 then 'high'
  when 3 then 'medium'
  else 'normal'
end
puts JSON.generate({
  is_npl: is_npl, severity: severity, kolektabilitas: kol,
  outstanding: outstanding, action: is_npl ? 'flag_for_collection' : 'pass'
})
