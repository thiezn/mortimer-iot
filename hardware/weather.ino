//! Weather module for Arduino Nano 33 IoT
#include <SPI.h>
#include <WiFiNINA.h>
#include <DHT.h>
#include "secrets.h"

int status = WL_IDLE_STATUS; // WiFi radio status

// --- Server Configuration ---
const char server[] = "pap.mortimer.nl"; // Target domain (DNS will resolve this)
const char resource[] = "/iot/weather";  // API endpoint path
const int port = 443;                    // Public HTTPS port

// Instantiate the secure WiFi client
WiFiSSLClient sslClient;

// --- DHT22 Configuration ---
#define DHTPIN 8      // Digital pin D8
#define DHTTYPE DHT22 // DHT 22 (AM2302)
DHT dht(DHTPIN, DHTTYPE);

// --- Timing ---
unsigned long previousMillis = 0;
const long interval = 5000; // Read and send every 5 seconds

bool ensureServerConnection()
{
  if (sslClient.connected())
  {
    return true;
  }

  sslClient.stop();

  if (sslClient.connect(server, port))
  {
    sslClient.setTimeout(1500);
    Serial.println("Connected to server (TLS encryption active, verification skipped).");
    return true;
  }

  return false;
}

void setup()
{
  // Initialize serial communication
  Serial.begin(9600);
  while (!Serial)
  {
    ; // Wait for serial port to connect (needed for native USB boards like Nano 33 IoT)
  }

  Serial.println("--- Arduino Nano 33 IoT Setup ---");

  // Initialize DHT sensor
  dht.begin();

  // Attempt to connect to WiFi network
  connectToWiFi();
}

void loop()
{
  // Check WiFi connection status and reconnect if dropped
  if (WiFi.status() != WL_CONNECTED)
  {
    Serial.println("WiFi connection lost. Reconnecting...");
    connectToWiFi();
  }

  unsigned long currentMillis = millis();

  // Non-blocking timer for reading the sensor
  if (currentMillis - previousMillis >= interval)
  {
    previousMillis = currentMillis;

    // Reading temperature or humidity takes about 250 milliseconds!
    float humidity = dht.readHumidity();
    // Read temperature as Celsius (the default)
    float temperature = dht.readTemperature();

    // Check if any reads failed and exit early (to try again).
    if (isnan(humidity) || isnan(temperature))
    {
      Serial.println(F("Failed to read from DHT sensor!"));
      return;
    }

    // Print results to the Serial Monitor
    Serial.print(F("Humidity: "));
    Serial.print(humidity);
    Serial.print(F("%  |  Temperature: "));
    Serial.print(temperature);
    Serial.println(F("°C"));

    // Send the measurements to the server via HTTPS POST
    sendMeasurements(temperature, humidity);
  }
}

void sendMeasurements(float temp, float hum)
{
  Serial.println("\n--- Sending Data via HTTPS POST ---");

  // 1. Construct the JSON payload string
  String jsonPayload = "{\"temperature\":" + String(temp, 2) + ",\"humidity\":" + String(hum, 2) + "}";

  // 2. Connect to the server only when needed so we can reuse the TLS session.
  if (ensureServerConnection())
  {
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

    // Empty line to signal the end of headers
    sslClient.println();

    // 4. Send the JSON payload body
    sslClient.print(jsonPayload);
    sslClient.flush();

    // 5. Read server response headers (for debugging)
    int statusCode = -1;
    int contentLength = -1;
    bool closeConnection = false;
    while (sslClient.connected())
    {
      String line = sslClient.readStringUntil('\n');
      if (line.startsWith("HTTP/1.1"))
      {
        Serial.print("Server Response Status: ");
        Serial.println(line);
        int firstSpace = line.indexOf(' ');
        if (firstSpace >= 0 && line.length() >= firstSpace + 4)
        {
          statusCode = line.substring(firstSpace + 1, firstSpace + 4).toInt();
        }
      }
      if (line.startsWith("Connection: close") || line.startsWith("connection: close"))
      {
        closeConnection = true;
      }
      if (line.startsWith("Content-Length: ") || line.startsWith("content-length: "))
      {
        contentLength = line.substring(15).toInt();
      }
      if (line == "\r")
      {
        // Headers are finished
        break;
      }
    }

    // Print response body if there is any
    String responseBody;
    if (contentLength > 0)
    {
      while (responseBody.length() < static_cast<unsigned int>(contentLength) && sslClient.connected())
      {
        while (sslClient.available() && responseBody.length() < static_cast<unsigned int>(contentLength))
        {
          responseBody += static_cast<char>(sslClient.read());
        }
      }
    }
    else
    {
      responseBody = sslClient.readString();
    }
    if (responseBody.length() > 0)
    {
      Serial.println("Response Body:");
      Serial.println(responseBody);
    }

    if (closeConnection)
    {
      sslClient.stop();
    }

    if (statusCode >= 200 && statusCode < 300)
    {
      Serial.println("Measurement accepted by server.");
    }
    else
    {
      Serial.print("Measurement rejected with status code: ");
      Serial.println(statusCode);
    }
  }
  else
  {
    Serial.println("Connection to server failed. If this persists, the server might require a specific TLS version.");
  }

  Serial.println("Measurement cycle complete.");
}

void connectToWiFi()
{
  if (WiFi.status() == WL_NO_MODULE)
  {
    Serial.println("Communication with WiFi module failed!");
    while (true)
      ;
  }

  String fv = WiFi.firmwareVersion();
  if (fv < WIFI_FIRMWARE_LATEST_VERSION)
  {
    Serial.println("Please upgrade the WiFiNINA firmware.");
  }

  while (WiFi.status() != WL_CONNECTED)
  {
    Serial.print("Attempting to connect to WPA SSID: ");
    Serial.println(ssid);
    status = WiFi.begin(ssid, pass);
    delay(5000); // Wait 5 seconds for connection
  }

  Serial.println("Connected to WiFi successfully!");
  printWiFiStatus();
}

void printWiFiStatus()
{
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
