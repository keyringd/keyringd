import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.ktor)
    application
}

repositories {
    mavenCentral()
    maven("https://jitpack.io")
}

dependencies {
    implementation(libs.ctapUsbip)
    implementation(libs.coroutines)
    implementation(libs.ktorServerCore)
    implementation(libs.ktorServerCio)
    implementation(libs.ktorServerNetty)
    implementation(libs.ktorServerWebsockets)
    implementation(libs.ktorServerContentNegotiation)
    implementation(libs.ktorSerializationJson)
    runtimeOnly(libs.slf4jSimple)
    testImplementation(libs.junitJupiter)
    testRuntimeOnly(libs.junitPlatformLauncher)
}

// Keep the thin JAR from overwriting Ktor's executable fat JAR when the
// project name and fatJar archiveFileName are the same.
tasks.jar {
    archiveClassifier.set("plain")
}

tasks.test {
    useJUnitPlatform()
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_17
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

application {
    mainClass = "com.keyringd.authenticator.app.MainKt"
}

ktor {
    fatJar {
        archiveFileName = "keyringd-authenticator-server.jar"
    }
}

layout.buildDirectory = rootProject.layout.projectDirectory.dir(
    "../../../target/authenticator-server"
)
