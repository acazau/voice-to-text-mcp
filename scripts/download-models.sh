#!/bin/bash

# Voice-to-Text MCP - Whisper Model Download Script
# This script helps users download appropriate Whisper models for their needs
# Compatible with Bash 3.2+ (macOS default)

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
MODELS_DIR="models"
BASE_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main"
DOWNLOAD_TOOL=""

# Model data - using simple arrays instead of associative arrays for compatibility
MODEL_NAMES=(
    "ggml-tiny.en.bin"
    "ggml-base.en.bin"
    "ggml-small.en.bin"
    "ggml-tiny.bin"
    "ggml-base.bin" 
    "ggml-small.bin"
    "ggml-medium.en.bin"
    "ggml-medium.bin"
    "ggml-large-v3.bin"
)

MODEL_SIZES=(
    "75MB"
    "142MB"
    "466MB"
    "75MB"
    "142MB"
    "466MB"
    "1.5GB"
    "1.5GB"
    "2.9GB"
)

MODEL_DESCRIPTIONS=(
    "English-only, fastest inference, good for development and testing"
    "English-only, best balance of speed and accuracy"
    "English-only, better accuracy, slower inference"
    "Multilingual, fastest inference"
    "Multilingual, good balance"
    "Multilingual, better accuracy"
    "English-only, high accuracy, requires more resources"
    "Multilingual, high accuracy, requires more resources"
    "Multilingual, highest accuracy, requires significant resources"
)

MODEL_CATEGORIES=(
    "development"
    "recommended"
    "high-quality"
    "multilingual-fast"
    "multilingual-balanced"
    "multilingual-quality"
    "production"
    "multilingual-production"
    "enterprise"
)

CATEGORY_NAMES=(
    "🚀 Development & Testing"
    "⭐ Recommended for Most Users"
    "🎯 High Quality English"
    "🌍 Multilingual (Fast)"
    "🌍 Multilingual (Balanced)"
    "🌍 Multilingual (High Quality)"
    "🏭 Production English"
    "🏭 Production Multilingual"
    "🏢 Enterprise (Maximum Quality)"
)

print_header() {
    echo -e "${CYAN}"
    echo "╭─────────────────────────────────────────────────────────────╮"
    echo "│           Voice-to-Text MCP - Model Downloader             │"
    echo "│                                                             │"
    echo "│  Download Whisper models for speech-to-text transcription  │"
    echo "╰─────────────────────────────────────────────────────────────╯"
    echo -e "${NC}"
}

check_requirements() {
    echo -e "${BLUE}🔍 Checking requirements...${NC}"
    
    # Check for download tools
    if command -v wget >/dev/null 2>&1; then
        DOWNLOAD_TOOL="wget"
        echo -e "${GREEN}✓ wget found${NC}"
    elif command -v curl >/dev/null 2>&1; then
        DOWNLOAD_TOOL="curl"
        echo -e "${GREEN}✓ curl found${NC}"
    else
        echo -e "${RED}❌ Error: Neither wget nor curl found. Please install one of them.${NC}"
        exit 1
    fi
    
    # Create models directory
    if [ ! -d "$MODELS_DIR" ]; then
        mkdir -p "$MODELS_DIR"
        echo -e "${GREEN}✓ Created models directory${NC}"
    else
        echo -e "${GREEN}✓ Models directory exists${NC}"
    fi
    
    echo ""
}

check_disk_space() {
    local required_mb=$1
    local model_name=$2
    
    if command -v df >/dev/null 2>&1; then
        local available_kb=$(df "$MODELS_DIR" 2>/dev/null | tail -1 | awk '{print $4}' || echo "0")
        local available_mb=$((available_kb / 1024))
        
        if [ "$available_mb" -lt "$required_mb" ]; then
            echo -e "${RED}❌ Insufficient disk space for $model_name${NC}"
            echo -e "   Required: ${required_mb}MB, Available: ${available_mb}MB"
            return 1
        fi
    fi
    return 0
}

get_model_size_mb() {
    local size=$1
    # Extract number from size (e.g., "142MB" -> "142", "1.5GB" -> "1500")
    if [[ "$size" == *"GB"* ]]; then
        local gb_size=$(echo "$size" | sed 's/[^0-9.]*//g')
        # Convert GB to MB: multiply by 1000 (using shell arithmetic)
        if [[ "$gb_size" == "1.5" ]]; then
            echo "1500"
        elif [[ "$gb_size" == "2.9" ]]; then
            echo "2900"
        else
            # Default fallback for any other GB size
            echo "1000"
        fi
    else
        echo "$size" | sed 's/[^0-9]*//g'
    fi
}

model_exists() {
    local model=$1
    [ -f "$MODELS_DIR/$model" ]
}

get_file_size() {
    local file=$1
    if [ -f "$file" ]; then
        if command -v stat >/dev/null 2>&1; then
            # Try macOS/BSD stat first, then Linux stat
            stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo "0"
        else
            echo "0"
        fi
    else
        echo "0"
    fi
}

download_model() {
    local model=$1
    local url="$BASE_URL/$model"
    local output_path="$MODELS_DIR/$model"
    local temp_path="$output_path.tmp"
    
    echo -e "${BLUE}📥 Downloading $model...${NC}"
    echo -e "   Source: $url"
    echo -e "   Destination: $output_path"
    echo ""
    
    # Find model index to get size
    local model_index=-1
    for i in "${!MODEL_NAMES[@]}"; do
        if [[ "${MODEL_NAMES[$i]}" == "$model" ]]; then
            model_index=$i
            break
        fi
    done
    
    if [ "$model_index" -ge 0 ]; then
        local required_mb=$(get_model_size_mb "${MODEL_SIZES[$model_index]}")
        if ! check_disk_space "$((required_mb + 100))" "$model"; then
            return 1
        fi
    fi
    
    # Download with appropriate tool
    local download_success=false
    
    if [ "$DOWNLOAD_TOOL" = "wget" ]; then
        if wget --continue --progress=bar --show-progress -O "$temp_path" "$url" 2>&1; then
            download_success=true
        fi
    elif [ "$DOWNLOAD_TOOL" = "curl" ]; then
        if curl -L --continue-at - --progress-bar -o "$temp_path" "$url"; then
            download_success=true
        fi
    fi
    
    if [ "$download_success" = true ] && [ -f "$temp_path" ]; then
        mv "$temp_path" "$output_path"
        echo -e "${GREEN}✓ Successfully downloaded $model${NC}"
        
        # Show file size
        local file_size=$(get_file_size "$output_path")
        local size_mb=$((file_size / 1024 / 1024))
        echo -e "   File size: ${size_mb}MB"
        echo ""
        return 0
    else
        echo -e "${RED}❌ Failed to download $model${NC}"
        [ -f "$temp_path" ] && rm -f "$temp_path"
        echo ""
        return 1
    fi
}

show_existing_models() {
    echo -e "${BLUE}📂 Currently downloaded models:${NC}"
    local found_any=false
    
    for i in "${!MODEL_NAMES[@]}"; do
        local model="${MODEL_NAMES[$i]}"
        if model_exists "$model"; then
            local file_size=$(get_file_size "$MODELS_DIR/$model")
            local size_mb=$((file_size / 1024 / 1024))
            local description="${MODEL_DESCRIPTIONS[$i]}"
            echo -e "   ${GREEN}✓${NC} $model (${size_mb}MB) - $description"
            found_any=true
        fi
    done
    
    if [ "$found_any" = false ]; then
        echo -e "   ${YELLOW}No models downloaded yet${NC}"
    fi
    echo ""
}

show_model_menu() {
    echo -e "${BLUE}📋 Available Whisper Models:${NC}"
    echo ""
    
    local current_category=""
    local index=1
    
    # Show models grouped by category
    for target_category in "development" "recommended" "high-quality" "multilingual-fast" "multilingual-balanced" "multilingual-quality" "production" "multilingual-production" "enterprise"; do
        local category_shown=false
        
        for i in "${!MODEL_NAMES[@]}"; do
            local model="${MODEL_NAMES[$i]}"
            local size="${MODEL_SIZES[$i]}"
            local description="${MODEL_DESCRIPTIONS[$i]}"
            local category="${MODEL_CATEGORIES[$i]}"
            
            if [ "$category" = "$target_category" ]; then
                if [ "$category_shown" = false ]; then
                    # Find category display name
                    local category_name=""
                    for j in "${!MODEL_CATEGORIES[@]}"; do
                        if [ "${MODEL_CATEGORIES[$j]}" = "$target_category" ]; then
                            category_name="${CATEGORY_NAMES[$j]}"
                            break
                        fi
                    done
                    echo -e "${CYAN}$category_name${NC}"
                    category_shown=true
                fi
                
                local status=""
                if model_exists "$model"; then
                    status="${GREEN}[Downloaded]${NC}"
                fi
                
                echo -e "   ${YELLOW}$index)${NC} $model ($size) $status"
                echo -e "      $description"
                ((index++))
            fi
        done
        
        if [ "$category_shown" = true ]; then
            echo ""
        fi
    done
    
    echo -e "${YELLOW}0)${NC} Exit"
    echo ""
    
    while true; do
        echo -n -e "${BLUE}Select a model to download (0-$((index-1))): ${NC}"
        read -r choice
        
        if [ "$choice" = "0" ]; then
            echo -e "${CYAN}👋 Goodbye!${NC}"
            exit 0
        elif [ "$choice" -gt 0 ] && [ "$choice" -lt "$index" ]; then
            # Find the selected model
            local selected_model=""
            local current_index=1
            
            for target_category in "development" "recommended" "high-quality" "multilingual-fast" "multilingual-balanced" "multilingual-quality" "production" "multilingual-production" "enterprise"; do
                for i in "${!MODEL_NAMES[@]}"; do
                    local category="${MODEL_CATEGORIES[$i]}"
                    if [ "$category" = "$target_category" ]; then
                        if [ "$current_index" = "$choice" ]; then
                            selected_model="${MODEL_NAMES[$i]}"
                            break 2
                        fi
                        ((current_index++))
                    fi
                done
            done
            
            if [ -n "$selected_model" ]; then
                if model_exists "$selected_model"; then
                    echo -e "${YELLOW}⚠️  Model $selected_model is already downloaded.${NC}"
                    echo -n -e "${BLUE}Download again? (y/N): ${NC}"
                    read -r confirm
                    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
                        continue
                    fi
                fi
                
                if download_model "$selected_model"; then
                    echo -e "${GREEN}✅ $selected_model is ready to use!${NC}"
                    echo ""
                    echo -e "${BLUE}💡 Usage examples:${NC}"
                    echo -e "   ${CYAN}# MCP Server mode${NC}"
                    echo -e "   ./target/release/voice-to-text-mcp --mcp-server models/$selected_model"
                    echo ""
                    echo -e "   ${CYAN}# Interactive CLI mode${NC}"
                    echo -e "   ./target/release/voice-to-text-mcp models/$selected_model"
                    echo ""
                fi
                
                echo -n -e "${BLUE}Download another model? (y/N): ${NC}"
                read -r continue_choice
                if [[ ! "$continue_choice" =~ ^[Yy]$ ]]; then
                    break
                fi
                echo ""
            fi
        else
            echo -e "${RED}❌ Invalid choice. Please select a number between 0 and $((index-1)).${NC}"
        fi
    done
}

show_quick_recommendations() {
    echo -e "${BLUE}🎯 Quick Recommendations:${NC}"
    echo ""
    echo -e "${GREEN}For most users:${NC} ggml-base.en.bin (142MB)"
    echo -e "   Best balance of speed, accuracy, and size for English transcription"
    echo ""
    echo -e "${GREEN}For development/testing:${NC} ggml-tiny.en.bin (75MB)"
    echo -e "   Fastest inference, good for quick testing and development"
    echo ""
    echo -e "${GREEN}For high quality:${NC} ggml-small.en.bin (466MB)"
    echo -e "   Better accuracy when you have more resources available"
    echo ""
    echo -e "${GREEN}For multilingual:${NC} ggml-base.bin (142MB)"
    echo -e "   Good balance for non-English or mixed-language content"
    echo ""
}

main() {
    print_header
    check_requirements
    show_existing_models
    show_quick_recommendations
    
    echo -n -e "${BLUE}Would you like to see all available models? (Y/n): ${NC}"
    read -r show_all
    
    if [[ ! "$show_all" =~ ^[Nn]$ ]]; then
        echo ""
        show_model_menu
    fi
    
    echo -e "${GREEN}✨ All done! Check the models/ directory for your downloaded models.${NC}"
    echo -e "${BLUE}📖 See models/README.md for usage instructions.${NC}"
}

# Handle Ctrl+C gracefully
trap 'echo -e "\n${YELLOW}⚠️  Download interrupted by user${NC}"; exit 1' INT

# Run main function
main "$@"