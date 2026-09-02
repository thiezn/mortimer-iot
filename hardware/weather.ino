//! Unified Weather Station for Arduino Nano 33 IoT
#include <SPI.h>
#include <WiFiNINA.h>
#include <DHT.h>

int status = WL_IDLE_STATUS; // WiFi radio status

// --- Server & Network Configuration ---
const char server[] = "pap.mortimer.nl";   // Target domain
const char resource[] = "/api/v1/weather"; // API endpoint path
const char ssid[] = "FRITZ!Box 7490";
const char pass[] = "06114872644865906045";
const char apiKey[] = "qwerty";
const int port = 443; // Public HTTPS port

WiFiSSLClient sslClient;

// --- DHT22 Configuration ---
#define DHTPIN 8      // Digital pin D8
#define DHTTYPE DHT22 // DHT 22 (AM2302)
DHT dht(DHTPIN, DHTTYPE);

// --- Anemometer (KY-003) Configuration ---
const byte HALL_PIN = 2;               // KY-003 signal pin D2 (Interrupt)
volatile unsigned long pulseCount = 0; // Incremented by hardware interrupt
unsigned long lastPulseCount = 0;
unsigned long lastWindCalcTime = 0;
float currentWindSpeedKmh = 0.0;       // Holds latest calculated wind speed

// --- Timers ---
unsigned long previousMillis = 0;
const long SEND_INTERVAL = 5000;       // Read DHT and send HTTPS payload every 5s
const long WIND_INTERVAL = 1000;       // Calculate wind speed every 1s

// --- Interrupt Service Routine (ISR) ---
void pulseISR() {
  pulseCount++;
}

bool ensureServerConnection() {
  if (sslClient.connected()) {
    return true;
  }

  sslClient.stop();

  if (sslClient.connect(server, port)) {
    sslClient.setTimeout(1500);
    Serial.println("Connected to server (TLS active).");
    return true;
  }

  return false;
}

void setup() {
  Serial.begin(9600);
  // Give Serial time to connect, but proceed after 3s timeout so board works standalone
  unsigned long startWait = millis();
  while (!Serial && (millis() - startWait < 3000));

  Serial.println("--- Arduino Nano 33 IoT Weather Station ---");

  // Initialize Anemometer Interrupt
  pinMode(HALL_PIN, INPUT_PULLUP);
  attachInterrupt(digitalPinToInterrupt(HALL_PIN), pulseISR, FALLING);
  Serial.println("Anemometer Initialized...");

  // Initialize DHT sensor
  dht.begin();

  // Connect to WiFi
  connectToWiFi();
}

void loop() {
  // Check WiFi connection status and reconnect if dropped
  if (WiFi.status() != WL_CONNECTED) {
    Serial.println("WiFi connection lost. Reconnecting...");
    connectToWiFi();
  }

  unsigned long currentMillis = millis();

  // 1. Calculate Wind Speed (every 1 second)
  if (currentMillis - lastWindCalcTime >= WIND_INTERVAL) {
    // Safely retrieve pulse count inside critical section
    noInterrupts();
    unsigned long pulses = pulseCount;
    interrupts();

    unsigned long deltaPulses = pulses - lastPulseCount;
    lastPulseCount = pulses;

    float timeElapsedSec = (currentMillis - lastWindCalcTime) / 1000.0;
    lastWindCalcTime = currentMillis;

    float rpm = (deltaPulses / timeElapsedSec) * 60.0;

    // Wind speed formula: speed km/h = RPM * 14 * 0.00377 * 1.31
    currentWindSpeedKmh = rpm * 14.0 * 0.00377 * 1.31;

    Serial.print("RPM: ");
    Serial.print(rpm, 1);
    Serial.print(" | Current Wind Speed: ");
    Serial.print(currentWindSpeedKmh, 2);
    Serial.println(" km/h");
  }

  // 2. Read Sensors and Send HTTPS POST (every 5 seconds)
  if (currentMillis - previousMillis >= SEND_INTERVAL) {
    previousMillis = currentMillis;

    float humidity = dht.readHumidity();
    float temperature = dht.readTemperature();

    if (isnan(humidity) || isnan(temperature)) {
      Serial.println(F("Failed to read from DHT sensor!"));
      return;
    }

    Serial.print(F("Humidity: "));
    Serial.print(humidity);
    Serial.print(F("%  |  Temperature: "));
    Serial.print(temperature);
    Serial.print(F("°C  |  Wind Speed: "));
    Serial.print(currentWindSpeedKmh);
    Serial.println(F(" km/h"));

    // Send payload including wind speed
    sendMeasurements(temperature, humidity, currentWindSpeedKmh);
  }
}

void sendMeasurements(float temp, float hum, float windSpeed) {
  Serial.println("\n--- Sending Data via HTTPS POST ---");

  // 1. Construct JSON payload with temp, humidity, and wind_speed
  String jsonPayload = "{\"temperature\":" + String(temp, 2) +
                       ",\"humidity\":" + String(hum, 2) +
                       ",\"wind_speed\":" + String(windSpeed, 2) + "}";

  // 2. Connect to the server
  if (ensureServerConnection()) {
    // 3. Send HTTP POST headers
    sslClient.print("POST ");
    sslClient.print(resource);
    sslClient.println(" HTTP/1.1");

    sslClient.print("Host: ");
    sslClient.println(server);

    sslClient.print("X-API-Key: ");
    sslClient.println(apiKey);
    sslClient.println("Content-Type: application/json");
    sslClient.println("Connection: keep-alive");

    sslClient.print("Content-Length: ");
    sslClient.println(jsonPayload.length());

    sslClient.println(); // Empty line signals end of headers

    // 4. Send payload
    sslClient.print(jsonPayload);
    sslClient.flush();

    // 5. Read response headers
    int statusCode = -1;
    int contentLength = -1;
    bool closeConnection = false;

    while (sslClient.connected()) {
      String line = sslClient.readStringUntil('\n');
      if (line.startsWith("HTTP/1.1")) {
        Serial.print("Server Response Status: ");
        Serial.println(line);
        int firstSpace = line.indexOf(' ');
        if (firstSpace >= 0 && line.length() >= firstSpace + 4) {
          statusCode = line.substring(firstSpace + 1, firstSpace + 4).toInt();
        }
      }
      if (line.startsWith("Connection: close") || line.startsWith("connection: close")) {
        closeConnection = true;
      }
      if (line.startsWith("Content-Length: ") || line.startsWith("content-length: ")) {
        contentLength = line.substring(15).toInt();
      }
      if (line == "\r") {
        break; // End of HTTP headers
      }
    }

    // Read response body
    String responseBody;
    if (contentLength > 0) {
      while (responseBody.length() < static_cast<unsigned int>(contentLength) && sslClient.connected()) {
        while (sslClient.available() && responseBody.length() < static_cast<unsigned int>(contentLength)) {
          responseBody += static_cast<char>(sslClient.read());
        }
      }
    } else {
      responseBody = sslClient.readString();
    }

    if (responseBody.length() > 0) {
      Serial.println("Response Body:");
      Serial.println(responseBody);
    }

    if (closeConnection) {
      sslClient.stop();
    }

    if (statusCode >= 200 && statusCode < 300) {
      Serial.println("Measurement accepted by server.");
    } else {
      Serial.print("Measurement rejected with status code: ");
      Serial.println(statusCode);
    }
  } else {
    Serial.println("Connection to server failed.");
  }

  Serial.println("Measurement cycle complete.");
}

void connectToWiFi() {
  if (WiFi.status() == WL_NO_MODULE) {
    Serial.println("Communication with WiFi module failed!");
    while (true);
  }

  String fv = WiFi.firmwareVersion();
  if (fv < WIFI_FIRMWARE_LATEST_VERSION) {
    Serial.println("Please upgrade the WiFiNINA firmware.");
  }

  while (WiFi.status() != WL_CONNECTED) {
    Serial.print("Attempting to connect to WPA SSID: ");
    Serial.println(ssid);
    status = WiFi.begin(ssid, pass);
    delay(5000);
  }

  Serial.println("Connected to WiFi successfully!");
  printWiFiStatus();
}

void printWiFiStatus() {
  Serial.print("SSID: ");
  Serial.println(WiFi.SSID());

  IPAddress ip = WiFi.localIP();
  Serial.print("IP Address: ");
  Serial.println(ip);

  long rssi = WiFi.RSSI();
  Serial.print("Signal strength (RSSI): ");
  Serial.print(rssi);
  Serial.println(" dBm");
}
