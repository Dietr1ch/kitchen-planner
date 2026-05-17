use axum_test::TestServer;
use kitchen_planner::io;
use kitchen_planner::models::cook::Cook;
use kitchen_planner::models::kitchen::Kitchen;
use kitchen_planner::models::recipe::Recipe;
use kitchen_planner::web::{AppState, create_router};
use serde_json::json;

fn test_server() -> TestServer {
	let state = AppState {
		default_kitchen: "data/kitchens/simple.ron".into(),
		default_cooks_dir: "data/cooks".into(),
		default_recipes_dir: "data/recipes".into(),
	};
	let app = create_router(state, "assets");
	TestServer::new(app)
}

#[tokio::test]
async fn defaults_returns_kitchen_cooks_recipes() {
	let server = test_server();

	let res = server.get("/api/defaults").await;
	res.assert_status_ok();

	let body: serde_json::Value = res.json();
	assert!(body.get("kitchen").is_some(), "missing kitchen");
	assert!(body.get("cooks").is_some(), "missing cooks");
	assert!(body.get("recipes").is_some(), "missing recipes");

	let recipes = body["recipes"].as_array().unwrap();
	let names: Vec<&str> = recipes
		.iter()
		.map(|r| r["name"].as_str().unwrap())
		.collect();
	assert!(names.contains(&"Lasagna"), "expected Lasagna");
	assert!(names.contains(&"Garlic Bread"), "expected Garlic Bread");
	assert!(
		names.contains(&"Mashed Potatoes"),
		"expected Mashed Potatoes"
	);
}

#[tokio::test]
async fn defaults_includes_equipment() {
	let server = test_server();

	let res = server.get("/api/defaults").await;
	res.assert_status_ok();

	let body: serde_json::Value = res.json();
	let equipment = body["kitchen"]["equipment"].as_array().unwrap();
	assert!(!equipment.is_empty(), "kitchen should have equipment");
	assert!(
		equipment.iter().any(|e| e["kind"] == "oven"),
		"should have an oven"
	);
}

#[tokio::test]
async fn plan_with_valid_data_returns_tasks() {
	let server = test_server();

	let kitchen: Kitchen = io::read_ron_file("data/kitchens/simple.ron").unwrap();
	let cooks: Vec<Cook> =
		io::read_ron_files(&["data/cooks/alice.ron", "data/cooks/bob.ron"]).unwrap();
	let recipes: Vec<Recipe> = io::read_ron_files(&["data/recipes/lasagna.ron"]).unwrap();

	let payload = json!({
		"kitchen": kitchen,
		"cooks": cooks,
		"recipes": recipes,
	});

	let res = server.post("/api/plan").json(&payload).await;
	res.assert_status_ok();

	let body: serde_json::Value = res.json();
	let tasks = body["tasks"].as_array().unwrap();
	assert!(!tasks.is_empty(), "plan should have tasks");

	let first = &tasks[0];
	assert!(first.get("id").is_some(), "task missing id");
	assert!(
		first.get("start_offset_minutes").is_some(),
		"task missing start"
	);
	assert!(
		first.get("duration_minutes").is_some(),
		"task missing duration"
	);
}

#[tokio::test]
async fn plan_with_all_cooks_and_recipes_succeeds() {
	let server = test_server();

	let kitchen: Kitchen = io::read_ron_file("data/kitchens/simple.ron").unwrap();
	let cooks: Vec<Cook> = io::read_ron_files(&[
		"data/cooks/alice.ron",
		"data/cooks/bob.ron",
		"data/cooks/charlie.ron",
		"data/cooks/diana.ron",
	])
	.unwrap();
	let recipes: Vec<Recipe> = io::read_ron_files(&[
		"data/recipes/lasagna.ron",
		"data/recipes/garlic-bread.ron",
		"data/recipes/mashed-potatoes.ron",
	])
	.unwrap();

	let payload = json!({
		"kitchen": kitchen,
		"cooks": cooks,
		"recipes": recipes,
	});

	let res = server.post("/api/plan").json(&payload).await;
	res.assert_status_ok();

	let body: serde_json::Value = res.json();
	let tasks = body["tasks"].as_array().unwrap();
	assert!(!tasks.is_empty(), "expected tasks for all recipes");
}

#[tokio::test]
async fn plan_with_no_cooks_fails() {
	let server = test_server();

	let kitchen: Kitchen = io::read_ron_file("data/kitchens/simple.ron").unwrap();
	let recipes: Vec<Recipe> = io::read_ron_files(&["data/recipes/lasagna.ron"]).unwrap();

	let payload = json!({
		"kitchen": kitchen,
		"cooks": [],
		"recipes": recipes,
	});

	let res = server.post("/api/plan").json(&payload).await;
	res.assert_status_bad_request();

	let body: serde_json::Value = res.json();
	let errors = body["errors"].as_array().unwrap();
	assert!(
		errors
			.iter()
			.any(|e| e["error_type"] == "no_cooks_for_task"),
		"expected 'no_cooks_for_task' error"
	);
}

#[tokio::test]
async fn plan_with_no_recipes_fails() {
	let server = test_server();

	let kitchen: Kitchen = io::read_ron_file("data/kitchens/simple.ron").unwrap();
	let cooks: Vec<Cook> = io::read_ron_files(&["data/cooks/alice.ron"]).unwrap();

	let payload = json!({
		"kitchen": kitchen,
		"cooks": cooks,
		"recipes": [],
	});

	let res = server.post("/api/plan").json(&payload).await;
	res.assert_status_bad_request();

	let body: serde_json::Value = res.json();
	let errors = body["errors"].as_array().unwrap();
	assert!(
		errors.iter().any(|e| e["error_type"] == "no_recipes"),
		"expected 'no_recipes' error"
	);
}

#[tokio::test]
async fn plan_without_required_equipment_fails() {
	let server = test_server();

	let mut kitchen: Kitchen = io::read_ron_file("data/kitchens/simple.ron").unwrap();
	// Remove all ovens so tasks needing one will fail
	kitchen.equipment.retain(|e| e.kind != "oven");
	let cooks: Vec<Cook> = io::read_ron_files(&["data/cooks/alice.ron"]).unwrap();
	let recipes: Vec<Recipe> = io::read_ron_files(&["data/recipes/lasagna.ron"]).unwrap();

	let payload = json!({
		"kitchen": kitchen,
		"cooks": cooks,
		"recipes": recipes,
	});

	let res = server.post("/api/plan").json(&payload).await;
	res.assert_status_bad_request();

	let body: serde_json::Value = res.json();
	let errors = body["errors"].as_array().unwrap();
	assert!(
		errors
			.iter()
			.any(|e| e["error_type"] == "missing_equipment_kind"),
		"expected 'missing_equipment_kind' error"
	);
}

#[tokio::test]
async fn plan_with_unskilled_cooks_fails() {
	let server = test_server();

	let kitchen: Kitchen = io::read_ron_file("data/kitchens/simple.ron").unwrap();
	// Bob has no knife_work skill, but Lasagna:s1 requires knife_work >= Novice
	let cooks: Vec<Cook> = io::read_ron_files(&["data/cooks/bob.ron"]).unwrap();
	let recipes: Vec<Recipe> = io::read_ron_files(&["data/recipes/lasagna.ron"]).unwrap();

	let payload = json!({
		"kitchen": kitchen,
		"cooks": cooks,
		"recipes": recipes,
	});

	let res = server.post("/api/plan").json(&payload).await;
	res.assert_status_bad_request();

	let body: serde_json::Value = res.json();
	let errors = body["errors"].as_array().unwrap();
	assert!(
		errors
			.iter()
			.any(|e| e["error_type"] == "cook_skill_insufficient"),
		"expected 'cook_skill_insufficient' error"
	);
}

#[tokio::test]
async fn index_html_is_served() {
	let server = test_server();

	let res = server.get("/").await;
	res.assert_status_ok();
}

#[tokio::test]
async fn static_files_are_served() {
	let server = test_server();

	let style = server.get("/style.css").await;
	style.assert_status_ok();

	let gantt_js = server.get("/gantt.js").await;
	gantt_js.assert_status_ok();

	let app_js = server.get("/app.js").await;
	app_js.assert_status_ok();
}

#[tokio::test]
async fn nonexistent_static_file_returns_404() {
	let server = test_server();

	let res = server.get("/nonexistent.css").await;
	res.assert_status_not_found();
}
