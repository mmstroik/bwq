# validate a query using the Brandwatch API
bw-validate query:
	curl -X POST https://api.brandwatch.com/query-validation \
		-H 'authorization: bearer {{BW_API_KEY}}' \
		-H 'Content-Type: application/json' \
		-d '{"booleanQuery": "{{query}}","languages": []}'
