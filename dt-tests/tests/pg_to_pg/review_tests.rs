#[cfg(test)]
mod test {

    use serial_test::serial;

    use crate::test_runner::test_base::TestBase;

    #[tokio::test]
    #[serial]
    async fn review_basic_test() {
        TestBase::run_review_test("pg_to_pg/review_basic_test").await;
    }
}
