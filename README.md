![image](https://github.com/user-attachments/assets/36be3a97-a277-466a-aff0-71c1525c8d5a)
### Startup
- You can run the app directly or start it via cargo providing postgres configuration string. But in this case you must have the database being configured and up. Example:
  ```bash
  cargo run -- --db-params="postgresql://user:password@postgres/db"
  ```
- Another option (recommended) is to start it up via docker-compose. In root folder:
  ```bash
  docker compose up -d
  ```

### Testing
- unit tests can be run with cargo:
```bash
cargo t
```
- if the app is up and run, you can call its API via scripts in _./scripts_ folder:
  - create an order:
    ```bash
    sh ./scripts/post_order.sh
    ```
  - get create order:
    ```bach
    sh ./scripts/get_order.sh
    ```
  - test with any other order_id:
    ```bash
    sh ./scripts/get_order.sh another_order_id_for_testing_not_found
    ```
