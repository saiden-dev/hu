Run NRQL query.

```bash
hu newrelic query "SELECT * FROM Transaction LIMIT 10"
hu newrelic query "SELECT count(*) FROM Transaction" -j
```
