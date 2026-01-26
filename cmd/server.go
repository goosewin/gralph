package cmd

import (
	"context"
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"

	"github.com/goosewin/gralph/internal/server"
	"github.com/spf13/cobra"
)

var (
	serverHost  string
	serverPort  int
	serverToken string
	serverOpen  bool
)

var serverCmd = &cobra.Command{
	Use:   "server",
	Short: "Start the HTTP status server",
	RunE:  runServer,
}

func init() {
	defaultHost := envOrDefault("GRALPH_SERVER_HOST", "127.0.0.1")
	defaultPort := envIntOrDefault("GRALPH_SERVER_PORT", 8080)
	defaultToken := os.Getenv("GRALPH_SERVER_TOKEN")
	defaultOpen := envBoolOrDefault("GRALPH_SERVER_OPEN", false)

	serverCmd.Flags().StringVarP(&serverHost, "host", "H", defaultHost, "Host/IP to bind to")
	serverCmd.Flags().IntVarP(&serverPort, "port", "p", defaultPort, "Port number")
	serverCmd.Flags().StringVarP(&serverToken, "token", "t", defaultToken, "Authentication token")
	serverCmd.Flags().BoolVar(&serverOpen, "open", defaultOpen, "Disable token requirement (use with caution)")

	rootCmd.AddCommand(serverCmd)
}

func runServer(cmd *cobra.Command, args []string) error {
	host := strings.TrimSpace(serverHost)
	if host == "" {
		host = "127.0.0.1"
	}
	if serverPort < 1 || serverPort > 65535 {
		return fmt.Errorf("invalid port number: %d", serverPort)
	}
	if !isLocalhost(host) && serverToken == "" && !serverOpen {
		return errors.New("token required when binding to non-localhost address (use --token or --open)")
	}
	if !isLocalhost(host) && serverOpen && serverToken == "" {
		fmt.Fprintln(os.Stderr, "Warning: server exposed without authentication (--open flag used)")
		fmt.Fprintln(os.Stderr, "Anyone with network access can view and control your sessions!")
	}

	printServerInfo(host, serverPort, serverToken)

	return server.StartServer(context.Background(), server.Options{
		Host:  host,
		Port:  serverPort,
		Token: serverToken,
		Open:  serverOpen,
	})
}

func printServerInfo(host string, port int, token string) {
	fmt.Printf("Starting gralph status server on %s:%d...\n", host, port)
	fmt.Println("Endpoints:")
	fmt.Println("  GET  /status        - Get all sessions")
	fmt.Println("  GET  /status/:name  - Get specific session")
	fmt.Println("  POST /stop/:name    - Stop a session")
	if strings.TrimSpace(token) != "" {
		fmt.Println("Authentication: Bearer token required")
	} else {
		fmt.Println("Authentication: None (use --token to enable)")
	}
	fmt.Println("")
	fmt.Println("Press Ctrl+C to stop")
	fmt.Println("")
}

func isLocalhost(host string) bool {
	switch host {
	case "127.0.0.1", "localhost", "::1":
		return true
	default:
		return false
	}
}

func envOrDefault(key, fallback string) string {
	value := strings.TrimSpace(os.Getenv(key))
	if value == "" {
		return fallback
	}
	return value
}

func envIntOrDefault(key string, fallback int) int {
	value := strings.TrimSpace(os.Getenv(key))
	if value == "" {
		return fallback
	}
	parsed, err := strconv.Atoi(value)
	if err != nil || parsed <= 0 {
		return fallback
	}
	return parsed
}

func envBoolOrDefault(key string, fallback bool) bool {
	value := strings.TrimSpace(strings.ToLower(os.Getenv(key)))
	if value == "" {
		return fallback
	}
	switch value {
	case "1", "true", "yes", "on":
		return true
	case "0", "false", "no", "off":
		return false
	default:
		return fallback
	}
}
