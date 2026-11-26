plugins {
    java
    id("io.mvnpm.gradle.plugin.native-java-plugin") version "1.0.0"
    id("io.quarkus") version "3.40.1.2"
}

group = "com.keyringd"
version = "1.0.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    implementation(enforcedPlatform("io.quarkus:quarkus-bom:3.40.1.2"))
    implementation("io.quarkiverse.roq:quarkus-roq:2.1.9")
    implementation("io.quarkiverse.roq:quarkus-roq-plugin-asciidoc-jruby:2.1.9")
}

java {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

tasks.withType<JavaCompile>().configureEach {
    options.encoding = "UTF-8"
    options.compilerArgs.add("-parameters")
}
