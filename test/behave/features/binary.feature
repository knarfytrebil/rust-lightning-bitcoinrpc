# -- File: features/binary.feature
Feature: Send sats between two directly connected nodes
    Scenario: Cli command should be able to start server
        Given Rustbolt and rbcli are both installed
        When We start 1 server
        Then Rustbolt instance should be initiated 

    Scenario: 
