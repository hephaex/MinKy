#!/bin/bash

# Deployment script for Minky application

set -e

echo "🚀 Starting Minky deployment..."

# Check if docker and docker-compose are installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Check if .env file exists
if [ ! -f .env ]; then
    echo "📝 Creating .env file from template..."
    cp .env.production .env
    echo "⚠️  Please edit .env file with your production settings before continuing."
    echo "   Update database credentials, secret keys, etc."
    read -p "Press enter when ready to continue..."
fi

# Build and start services
echo "🔨 Building Docker images..."
docker-compose build

echo "🗄️  Starting database..."
docker-compose up -d db

# Wait for database to be ready
echo "⏳ Waiting for database to be ready..."
sleep 10

# Run database migrations
echo "📊 Running database migrations..."
docker-compose run --rm app flask db upgrade

# Create performance indexes
echo "🔍 Creating database indexes..."
docker-compose run --rm app python -c "from app.utils.performance import create_performance_indexes; from app import create_app; app = create_app(); app.app_context().push(); create_performance_indexes()"

# Start all services
echo "🌟 Starting all services..."
docker-compose up -d

# Show status
echo "✅ Deployment complete!"
echo ""
echo "Services running:"
docker-compose ps

echo ""
echo "🌐 Application URLs:"
echo "   Frontend: http://localhost:3000"
echo "   Backend API: http://localhost:5000"
echo "   Database: localhost:5432"
echo ""
echo "📝 Logs:"
echo "   docker-compose logs -f        # All services"
echo "   docker-compose logs -f app    # Backend only"
echo "   docker-compose logs -f frontend # Frontend only"
echo ""
echo "🛑 To stop:"
echo "   docker-compose down"