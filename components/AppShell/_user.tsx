import React from 'react';
import {Avatar, Box, Button, Stack, Text, ThemeIcon} from '@mantine/core';
import {MdAccountCircle, MdArrowRight} from "react-icons/md";
import Link from "next/link";
import {useUser} from "@supabase/auth-helpers-react";
import useProfileData from "../../hooks/useProfileData";

export default function UserProfile() {
    const user = useUser()
    const profileData = useProfileData()

    return <Box
        pt={'sm'}
        style={{
            borderTop: `1px solid var(--mantine-color-dark-4)`,
        }}
        component={Link} href='/profile' passHref={true}>
        <Button
            leftSection={
                <>
                    {profileData.profile?.name ?
                        <Avatar
                            src={`https://ui-avatars.com/api/?name=${profileData.profile?.name}&background=random&rounded=true`}
                            radius="xl"
                        />
                        :
                        <ThemeIcon radius={"xl"} variant={'outline'} color={'purple'} size={42} p={2}>
                            <MdAccountCircle size={40}/>
                        </ThemeIcon>
                    }
                </>
            }
            rightSection={<MdArrowRight size={20}/>}
            fullWidth={true}
            p={'xs'}
            radius={'sm'}
            size={'xl'}
            variant={'subtle'}
            color={'gray'}
            justify={'start'}>
            <Stack gap={0} align={'flex-start'}>
                <Text size="sm" fw={500}>
                    {profileData.profile ? (profileData.profile?.name || "Profil") : "Logare"}
                </Text>
                {user?.email &&
                    <Text c="dimmed" size="xs">
                        {user?.email}
                    </Text>
                }
            </Stack>
        </Button>
    </Box>
}
