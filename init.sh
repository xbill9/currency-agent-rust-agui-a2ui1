#!/bin/bash

# --- Function for error handling ---
handle_error() {
  echo -e "\n\n*******************************************************"
  echo "Error: $1"
  echo "*******************************************************"
}

# --- Part 0: Verify gcloud authentication ---
echo "--- Verifying Google Cloud Authentication ---"
if ! gcloud auth list --filter=status:ACTIVE --format="value(account)" 2>/dev/null | grep -q "@"; then
    echo "No active gcloud account found. Please authenticate."
    gcloud auth login || handle_error "gcloud authentication failed."
else
    echo "gcloud is authenticated: $(gcloud auth list --filter=status:ACTIVE --format="value(account)")"
fi

# --- Part 1: Find or Create Google Cloud Project ID ---
PROJECT_FILE="$HOME/project_id.txt"
PROJECT_ID_SET=false

# Check if a project ID file already exists and points to a valid project
if [ -s "$PROJECT_FILE" ]; then
    EXISTING_PROJECT_ID=$(cat "$PROJECT_FILE" | tr -d '[:space:]') # Read and trim whitespace
    echo "--- Found existing project ID in $PROJECT_FILE: $EXISTING_PROJECT_ID ---"
    echo "Verifying this project exists in Google Cloud..."

    # Check if the project actually exists in GCP and we have permission to see it
    if gcloud projects describe "$EXISTING_PROJECT_ID" --quiet >/dev/null 2>&1; then
        echo "Project '$EXISTING_PROJECT_ID' successfully verified."
        FINAL_PROJECT_ID=$EXISTING_PROJECT_ID
        PROJECT_ID_SET=true

        # Ensure gcloud config is set to this project for the current session
        gcloud config set project "$FINAL_PROJECT_ID" || handle_error "Failed to set active project to '$FINAL_PROJECT_ID'."
        echo "Set active gcloud project to '$FINAL_PROJECT_ID'."
    else
        echo "Warning: Project '$EXISTING_PROJECT_ID' from file does not exist or you lack permissions."
        echo "Removing invalid reference file and proceeding with new project creation."
        rm "$PROJECT_FILE"
    fi
fi

# If no valid existing project was found, start the interactive creation process
if [ "$PROJECT_ID_SET" = false ]; then
    echo "--- Creating and Setting New Google Cloud Project ID ---"
    CODELAB_PROJECT_PREFIX="production-ready-ai"

    # Dynamic Length Calculation
    PREFIX_LEN=${#CODELAB_PROJECT_PREFIX}
    if (( PREFIX_LEN > 25 )); then
      handle_error "The project prefix '$CODELAB_PROJECT_PREFIX' is too long (${PREFIX_LEN} chars). Maximum is 25."
    fi
    MAX_SUFFIX_LEN=$(( 30 - PREFIX_LEN - 1 ))
    echo "Project prefix '${CODELAB_PROJECT_PREFIX}' is ${PREFIX_LEN} chars. Suffix will be ${MAX_SUFFIX_LEN} chars."

    # Loop until a project is successfully created.
    while true; do
      RANDOM_SUFFIX=$(LC_ALL=C tr -dc 'a-z0-9' < /dev/urandom | head -c "$MAX_SUFFIX_LEN")
      SUGGESTED_PROJECT_ID="${CODELAB_PROJECT_PREFIX}-${RANDOM_SUFFIX}"

      read -p "Enter project ID [default: $SUGGESTED_PROJECT_ID]: " FINAL_PROJECT_ID

      if [[ -z "$FINAL_PROJECT_ID" ]]; then
          FINAL_PROJECT_ID="$SUGGESTED_PROJECT_ID"
      fi

      # Check if the project already exists and is accessible to the user
      if gcloud projects describe "$FINAL_PROJECT_ID" --quiet >/dev/null 2>&1; then
        echo "Project '$FINAL_PROJECT_ID' already exists and is accessible."
        gcloud config set project "$FINAL_PROJECT_ID" || handle_error "Failed to set active project to $FINAL_PROJECT_ID."
        echo "Set active gcloud project to $FINAL_PROJECT_ID."
        echo "$FINAL_PROJECT_ID" > "$PROJECT_FILE" || handle_error "Failed to save project ID to $PROJECT_FILE."
        echo "Successfully saved project ID to $PROJECT_FILE."
        break
      fi

      echo "Attempting to create project with ID: $FINAL_PROJECT_ID"
      ERROR_OUTPUT=$(gcloud projects create "$FINAL_PROJECT_ID" --quiet 2>&1)
      CREATE_STATUS=$?

      if [[ $CREATE_STATUS -eq 0 ]]; then
        echo "Successfully created project: $FINAL_PROJECT_ID"
        gcloud config set project "$FINAL_PROJECT_ID" || handle_error "Failed to set active project to $FINAL_PROJECT_ID."
        echo "Set active gcloud project to $FINAL_PROJECT_ID."
        echo "$FINAL_PROJECT_ID" > "$PROJECT_FILE" || handle_error "Failed to save project ID to $PROJECT_FILE."
        echo "Successfully saved project ID to $PROJECT_FILE."
        break
      else
        echo "Could not create project '$FINAL_PROJECT_ID'."
        echo "Reason from gcloud: $ERROR_OUTPUT"
        echo -e "This ID may be taken. Please try a different project ID.\n"
      fi
    done
fi


echo -e "\n--- Full Setup Complete ---"
