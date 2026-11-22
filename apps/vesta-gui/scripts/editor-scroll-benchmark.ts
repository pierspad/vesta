import { mount } from "svelte";
import "../src/app.css";
import Benchmark from "../src/lib/test/fixtures/EditorScrollBenchmark.svelte";
mount(Benchmark, {target: document.getElementById("app")!});
