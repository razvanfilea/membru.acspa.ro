import {ActionIcon, Button, Card, Center, Checkbox, Group, Loader, Paper, Stack, Text, Title} from "@mantine/core";
import React, {useEffect, useMemo, useState} from "react";
import {NextRouter, useRouter} from "next/router";
import ReservationComponent from "../components/Reservation";
import {Location, Profile, Reservation} from "../types/wrapper";
import {MdLogout, MdPassword, MdRefresh} from "react-icons/md";
import {isReservationCancelable} from "../utils/date";
import {Database} from "../types/database.types";
import {SupabaseClient, useSupabaseClient} from "@supabase/auth-helpers-react";
import {useListState} from "@mantine/hooks";
import {UserProfileLayout} from "../components/UserProfileLayout";
import Link from "next/link";
import useProfileData from "../hooks/useProfileData";
import {createPagesBrowserClient} from "@supabase/auth-helpers-nextjs";

async function signOut(supabase: SupabaseClient<Database>, router: NextRouter) {
    const {error} = await supabase.auth.signOut()

    if (error == null) {
        await router.push("/login")
    } else {
        console.log('Auth', error);
        console.log(error); // Failure
    }
}

function fetchReservations(
    supabase: SupabaseClient<Database>,
    profile: Profile,
    setReservations: (data: Reservation[]) => void,
) {
    supabase.from('rezervari')
        .select()
        .eq("user_id", profile.id)
        .order('start_date', {ascending: false})
        .order('start_hour', {ascending: true})
        .then(res => {
            setReservations(res.data || [])
        })
}

interface IParams {
    locations: Location[]
}

export default function ProfilePage({locations}: IParams) {
    const supabase = useSupabaseClient<Database>()
    const router = useRouter()
    const profileData = useProfileData()

    const [reservations, reservationsHandle] = useListState<Reservation>([])
    const [showCancelled, setShowCancelled] = useState(false)

    useEffect(() => {
        if (!profileData.isLoading && profileData.profile == null) {
            const timer = setTimeout(() => {
                router.push('/login').then(null)
            }, 400)

            return () => clearTimeout(timer)
        }
    }, [profileData, router])

    useEffect(() => {
        if (profileData.profile == null)
            return;

        fetchReservations(supabase, profileData.profile, reservationsHandle.setState)
    }, [profileData, reservationsHandle.setState, supabase])

    const filteredReservations = useMemo(() => {
        return reservations.filter((res) => showCancelled || !res.cancelled)
    }, [reservations, showCancelled])

    if (profileData.isLoading)
        return <Center> <Loader/> </Center>;

    if (profileData.profile == null)
        return (<></>)

    return (<>
        <Paper shadow={"md"} p={'lg'} sx={(theme) => ({
            backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
            margin: theme.spacing.lg,
            '@media (max-width: 900px)': {
                margin: theme.spacing.xs,
            },
        })}>

            <Group position={"apart"}>
                <UserProfileLayout profile={profileData.profile}/>

                <Group>
                    <Link href={'password_recovery'}>
                        <Button leftIcon={<MdPassword/>} variant={"outline"} color={'blue'}>Schimbă parola</Button>
                    </Link>

                    <Button leftIcon={<MdLogout/>} variant={"outline"} color={'red'}
                            onClick={() => signOut(supabase, router)}>Sign out</Button>
                </Group>
            </Group>

        </Paper>

        <Stack sx={(theme) => ({
            padding: theme.spacing.lg,
            '@media (max-width: 900px)': {
                paddingLeft: theme.spacing.md,
                paddingRight: theme.spacing.md,
            },
            '@media (max-width: 600px)': {
                paddingLeft: 0,
                paddingRight: 0,
            }
        })}>
            <Group position={'apart'}>
                <Title order={2}>Rezervările tale:</Title>

                <ActionIcon variant={'light'} radius={'xl'} size={36}
                            onClick={() => fetchReservations(supabase, profileData.profile!, reservationsHandle.setState)}>
                    <MdRefresh size={28}/>
                </ActionIcon>
            </Group>

            <Checkbox
                label="Afișează anulate"
                checked={showCancelled}
                onChange={(event) => setShowCancelled(event.currentTarget.checked)}/>

            {reservations.length == 0 &&
                <Text size={"lg"}>Nu ai nicio rezervare</Text>
            }

            {filteredReservations.map((reservation) => (
                <Card key={reservation.id}>
                    {ReservationComponent(
                        reservation,
                        locations.find(value => value.name == reservation.location)!,
                        true,
                        (isReservationCancelable(reservation)) ? (async () => {
                            const newData: Reservation = {
                                ...reservation,
                                cancelled: true
                            }

                            const {data} = await supabase.from('rezervari')
                                .update(newData)
                                .eq('id', reservation.id)
                                .select()

                            if (data) {
                                reservationsHandle.filter(value => value.id != data![0].id)
                                reservationsHandle.append(data![0])
                            }
                        }) : null
                    )}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const supabase = createPagesBrowserClient<Database>()

    const {data: locations} = await supabase.from('locations').select()
    const props: IParams = {
        locations: locations!
    }

    return {props}
}
