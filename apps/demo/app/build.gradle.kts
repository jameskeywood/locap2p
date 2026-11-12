plugins {
    application
    checkstyle
}

repositories {
    mavenCentral()
}

dependencies {
    implementation(libs.jackson.databind)

    testImplementation(libs.junit.jupiter)

    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
}

application {
    mainClass = "com.locap2p.demo.Main"
}

checkstyle {
    toolVersion = "14.4.0"
}

tasks.named<Test>("test") {
    useJUnitPlatform()
}

tasks.named<JavaExec>("run") {
    standardInput = System.`in`
}
