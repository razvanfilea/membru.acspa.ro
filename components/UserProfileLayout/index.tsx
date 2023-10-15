import {Profile} from "../../types/wrapper";
import {Avatar, Group, Stack, Text} from "@mantine/core";
import {MdVpnKey} from "react-icons/md";
import React from "react";

interface IParams {
    profile: Profile
}

export function UserProfileLayout({profile}: IParams) {
    return <Group wrap={'nowrap'}>
        <Avatar
            src={`https://ui-avatars.com/api/?name=${profile.name}&background=random&rounded=true&format=svg`}
            radius={"md"}/>

        <Stack gap={1}>
            <Text size="md" fw={500}>{profile.name}</Text>
            <Text size="sm">{profile.role}</Text>
        </Stack>

        {profile.has_key &&
            <MdVpnKey size="1.125rem"/>
        }
    </Group>
}
